#!/usr/bin/env node
// Единственное место, которое знает, из каких файлов складывается версия лаунчера.
// Платформенные обёртки (set-version.ps1 и set-version.sh) только зовут этот файл,
// чтобы две копии логики не разъехались.
//
//   node scripts/set-version.mjs            показать текущие версии
//   node scripts/set-version.mjs 1.5.0      проставить новую везде
//   node scripts/set-version.mjs 1.5.0 -n   показать, что изменится, но не писать

import {readFile, writeFile} from "node:fs/promises"
import {dirname, join} from "node:path"
import {fileURLToPath} from "node:url"

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..")

// Версия обязана быть строгой X.Y.Z: с суффиксами вроде -beta1 не собирается
// установщик NSIS и ломается сравнение версий в автообновлении.
const SEMVER = /^\d+\.\d+\.\d+$/

// Каждое правило - регулярка ровно с тремя группами: до версии, сама версия, после.
// Правило, которое ничего не нашло, считается ошибкой: молча пропущенный файл -
// это ровно тот случай, ради которого скрипт и написан.
const TARGETS = [
    {
        file: "package.json",
        rules: [/(^ {2}"version": ")([^"]+)(")/m]
    },
    {
        file: "package-lock.json",
        rules: [
            /(^ {2}"version": ")([^"]+)(")/m,
            // Корневой пакет внутри "packages" лежит под пустым ключом,
            // у всех остальных там свои версии - их трогать нельзя.
            /(""\s*:\s*\{[\s\S]*?"version": ")([^"]+)(")/
        ]
    },
    {
        file: "src-tauri/tauri.conf.json",
        rules: [/(^ {2}"version": ")([^"]+)(")/m]
    },
    {
        file: "src-tauri/Cargo.toml",
        rules: [/(\[package\][\s\S]*?\bversion\s*=\s*")([^"]+)(")/]
    },
    {
        file: "src-tauri/core/Cargo.toml",
        rules: [/(\[package\][\s\S]*?\bversion\s*=\s*")([^"]+)(")/]
    },
    {
        file: "src-tauri/Cargo.lock",
        // Только свои крейты: в блокировке полно чужих пакетов с такой же версией.
        rules: ["cast-launcher", "cast-core"].map(crate =>
            new RegExp(`(name = "${crate}"[\\s\\S]*?\\bversion = ")([^"]+)(")`)
        )
    }
]

async function main() {
    const args = process.argv.slice(2)
    const dryRun = args.some(arg => arg === "-n" || arg === "--dry-run")
    const version = args.find(arg => !arg.startsWith("-"))

    const files = await Promise.all(TARGETS.map(read))

    if (!version) {
        report(files)
        console.log("\nЧтобы сменить: node scripts/set-version.mjs <версия>")
        return
    }

    if (!SEMVER.test(version)) {
        fail(`Версия должна быть в формате X.Y.Z, а не "${version}"`)
    }

    const changed = files.filter(file => file.versions.some(found => found !== version))

    if (!changed.length) {
        console.log(`Везде уже ${version}, менять нечего.`)
        return
    }

    report(files, version)

    if (dryRun) {
        console.log("\n--dry-run: ничего не записано.")
        return
    }

    for (const file of changed) {
        await writeFile(join(ROOT, file.target.file), replaced(file, version))
    }

    console.log(`\nГотово: ${version} проставлена в ${changed.length} файл(ах).`)
    console.log("Cargo.lock и package-lock.json обновлены здесь же, пересобирать их не нужно.")
}

async function read(target) {
    const path = join(ROOT, target.file)
    const text = await readFile(path, "utf8").catch(error => {
        fail(`Не удалось прочитать ${target.file}: ${error.message}`)
    })

    const versions = target.rules.map(rule => {
        const found = text.match(rule)

        if (!found) {
            fail(`В ${target.file} не нашлось версии - формат файла изменился, поправьте правило в scripts/set-version.mjs`)
        }

        return found[2]
    })

    return {target, text, versions}
}

function replaced(file, version) {
    return file.target.rules.reduce(
        (text, rule) => text.replace(rule, (_, before, __, after) => before + version + after),
        file.text
    )
}

function report(files, version) {
    const width = Math.max(...files.map(file => file.target.file.length))
    const current = new Set(files.flatMap(file => file.versions))

    for (const file of files) {
        const from = [...new Set(file.versions)].join(", ")
        const arrow = version && file.versions.some(found => found !== version) ? ` -> ${version}` : ""

        console.log(`  ${file.target.file.padEnd(width)}  ${from}${arrow}`)
    }

    if (current.size > 1) {
        console.log(`\nВнимание: до правки версии разошлись (${[...current].join(", ")}).`)
    }
}

function fail(message) {
    console.error(`Ошибка: ${message}`)
    process.exit(1)
}

await main()
