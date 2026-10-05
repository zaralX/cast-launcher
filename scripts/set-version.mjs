#!/usr/bin/env node
// The only place that knows which files carry the launcher version.
// The platform wrappers (set-version.ps1 and set-version.sh) just call this file,
// so the logic has a single copy.
//
//   node scripts/set-version.mjs            show the current versions
//   node scripts/set-version.mjs 1.5.0      set a new one everywhere
//   node scripts/set-version.mjs 1.5.0 -n   show what would change without writing

import { readFile, writeFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..')

// The version must be a strict X.Y.Z: with suffixes like -beta1 the NSIS installer
// does not build and the updater's version comparison breaks.
const SEMVER = /^\d+\.\d+\.\d+$/

// Every rule is a regex with exactly three groups: before the version, the version, after it.
// A rule that finds nothing is an error: a silently skipped file is exactly the case
// this script exists for.
const TARGETS = [
  {
    file: 'package.json',
    rules: [/(^ {2}"version": ")([^"]+)(")/m],
  },
  {
    file: 'package-lock.json',
    rules: [
      /(^ {2}"version": ")([^"]+)(")/m,
      // The root package inside "packages" sits under the empty key,
      // every other entry there has its own version and must stay untouched.
      /(""\s*:\s*\{[\s\S]*?"version": ")([^"]+)(")/,
    ],
  },
  {
    file: 'src-tauri/tauri.conf.json',
    rules: [/(^ {2}"version": ")([^"]+)(")/m],
  },
  {
    file: 'src-tauri/Cargo.toml',
    rules: [/(\[package\][\s\S]*?\bversion\s*=\s*")([^"]+)(")/],
  },
  {
    file: 'src-tauri/core/Cargo.toml',
    rules: [/(\[package\][\s\S]*?\bversion\s*=\s*")([^"]+)(")/],
  },
  {
    file: 'src-tauri/Cargo.lock',
    // Only our own crates: the lock file is full of other packages with the same version.
    rules: ['cast-launcher', 'cast-core'].map(crate =>
      new RegExp(`(name = "${crate}"[\\s\\S]*?\\bversion = ")([^"]+)(")`),
    ),
  },
]

async function main() {
  const args = process.argv.slice(2)
  const dryRun = args.some(arg => arg === '-n' || arg === '--dry-run')
  const version = args.find(arg => !arg.startsWith('-'))

  const files = await Promise.all(TARGETS.map(read))

  if (!version) {
    report(files)
    console.log('\nTo change it: node scripts/set-version.mjs <version>')
    return
  }

  if (!SEMVER.test(version)) {
    fail(`The version must look like X.Y.Z, got "${version}"`)
  }

  const changed = files.filter(file => file.versions.some(found => found !== version))

  if (!changed.length) {
    console.log(`Already ${version} everywhere, nothing to change.`)
    return
  }

  report(files, version)

  if (dryRun) {
    console.log('\n--dry-run: nothing was written.')
    return
  }

  for (const file of changed) {
    await writeFile(join(ROOT, file.target.file), replaced(file, version))
  }

  console.log(`\nDone: ${version} is set in ${changed.length} file(s).`)
  console.log('Cargo.lock and package-lock.json are updated here as well, no need to regenerate them.')
}

async function read(target) {
  const path = join(ROOT, target.file)
  const text = await readFile(path, 'utf8').catch((error) => {
    fail(`Failed to read ${target.file}: ${error.message}`)
  })

  const versions = target.rules.map((rule) => {
    const found = text.match(rule)

    if (!found) {
      fail(`No version found in ${target.file}: the file format changed, fix the rule in scripts/set-version.mjs`)
    }

    return found[2]
  })

  return { target, text, versions }
}

function replaced(file, version) {
  return file.target.rules.reduce(
    (text, rule) => text.replace(rule, (_, before, __, after) => before + version + after),
    file.text,
  )
}

function report(files, version) {
  const width = Math.max(...files.map(file => file.target.file.length))
  const current = new Set(files.flatMap(file => file.versions))

  for (const file of files) {
    const from = [...new Set(file.versions)].join(', ')
    const arrow = version && file.versions.some(found => found !== version) ? ` -> ${version}` : ''

    console.log(`  ${file.target.file.padEnd(width)}  ${from}${arrow}`)
  }

  if (current.size > 1) {
    console.log(`\nWarning: the versions had already diverged (${[...current].join(', ')}).`)
  }
}

function fail(message) {
  console.error(`Error: ${message}`)
  process.exit(1)
}

await main()
