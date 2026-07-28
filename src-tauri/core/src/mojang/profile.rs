use std::path::PathBuf;

use serde::Serialize;

use crate::mojang::rules::{check_rules, Features, RuntimeContext};
use crate::mojang::version::{
    Argument, AssetIndexRef, JavaVersionSpec, Library, LibraryArtifact, MojangArtifact, VersionPackage,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedArtifact {
    pub path: String,
    pub url: Option<String>,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

impl ResolvedArtifact {
    fn from_library_artifact(artifact: &LibraryArtifact) -> Self {
        Self {
            path: artifact.path.clone(),
            url: artifact.url.as_deref().map(str::trim).filter(|url| !url.is_empty()).map(str::to_string),
            sha1: artifact.sha1.clone(),
            size: artifact.size,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedLibrary {
    pub name: Option<String>,
    pub artifact: Option<ResolvedArtifact>,
    pub native: Option<ResolvedArtifact>,
}

impl ResolvedLibrary {
    pub fn artifacts(&self) -> impl Iterator<Item = &ResolvedArtifact> {
        self.artifact.iter().chain(self.native.iter())
    }
}

pub fn resolve_libraries(libraries: &[Library], ctx: &RuntimeContext) -> Vec<ResolvedLibrary> {
    let features = Features::new();
    let mut resolved = Vec::with_capacity(libraries.len());

    for library in libraries {
        if !check_rules(library.rules.as_deref(), ctx, &features) {
            continue;
        }

        let artifact = resolve_artifact(library);
        let native = resolve_native(library, ctx);

        if artifact.is_none() && native.is_none() {
            continue;
        }

        resolved.push(ResolvedLibrary {
            name: library.name.clone(),
            artifact,
            native,
        });
    }

    resolved
}

const DEFAULT_MAVEN: &str = "https://libraries.minecraft.net/";

pub fn resolve_artifact(library: &Library) -> Option<ResolvedArtifact> {
    library
        .downloads
        .as_ref()
        .and_then(|downloads| downloads.artifact.as_ref())
        .map(ResolvedArtifact::from_library_artifact)
        .or_else(|| resolve_by_coordinate(library))
}

fn resolve_by_coordinate(library: &Library) -> Option<ResolvedArtifact> {
    if library.downloads.is_some() || library.natives.is_some() {
        return None;
    }

    let name = library.name.as_deref()?;
    let gradle = crate::mojang::maven::Gradle::parse(name).ok()?;
    let repository = library.url.as_deref().unwrap_or(DEFAULT_MAVEN);

    Some(ResolvedArtifact {
        path: gradle.path(),
        url: Some(gradle.url(repository)),
        sha1: None,
        size: None,
    })
}

fn resolve_native(library: &Library, ctx: &RuntimeContext) -> Option<ResolvedArtifact> {
    let classifier = library.natives.as_ref()?.get(ctx.os.as_str())?;
    let classifier = classifier.replace("${arch}", ctx.bits());

    let artifact = library
        .downloads
        .as_ref()?
        .classifiers
        .as_ref()?
        .get(&classifier)?;

    Some(ResolvedArtifact::from_library_artifact(artifact))
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaRequirement {
    pub major: Option<u32>,
    pub component: Option<String>,
    pub at_least: bool,
}

impl JavaRequirement {
    pub fn from_package(package: &VersionPackage) -> Self {
        match &package.java_version {
            Some(JavaVersionSpec {
                component,
                major_version,
            }) => Self {
                major: Some(*major_version),
                component: Some(component.clone()),
                at_least: false,
            },
            None => Self::from_minecraft_version(&package.id),
        }
    }

    pub fn from_minecraft_version(version: &str) -> Self {
        let numbers: Vec<u32> = version
            .split(|c: char| !c.is_ascii_digit())
            .filter(|part| !part.is_empty())
            .filter_map(|part| part.parse().ok())
            .collect();

        let (Some(major), minor, patch) = (
            numbers.first().copied(),
            numbers.get(1).copied().unwrap_or(0),
            numbers.get(2).copied().unwrap_or(0),
        ) else {
            return Self::default();
        };

        if major >= 20 {
            return Self {
                major: Some(21),
                component: None,
                at_least: true,
            };
        }

        if major != 1 {
            return Self::default();
        }

        let required = match minor {
            m if m >= 21 => 21,
            20 if patch >= 5 => 21,
            20 => 17,
            m if m >= 18 => 17,
            17 => 16,
            _ => 8,
        };

        Self {
            major: Some(required),
            component: None,
            at_least: false,
        }
    }

    pub fn describe(&self) -> String {
        match self.major {
            Some(major) if self.at_least => format!("Java {major} или новее"),
            Some(major) => format!("Java {major}"),
            None => "любая Java".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedProfile {
    pub version_id: String,
    pub version_type: String,
    pub main_class: String,
    pub assets_id: String,
    pub asset_index: Option<AssetIndexRef>,
    pub client_download: Option<MojangArtifact>,
    pub libraries: Vec<ResolvedLibrary>,
    pub main_jar: PathBuf,
    pub java: JavaRequirement,
    #[serde(skip)]
    pub arguments: ResolvedArguments,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedArguments {
    pub game: Vec<Argument>,
    pub jvm: Vec<Argument>,
    pub legacy_game: Option<String>,
}

impl ResolvedArguments {
    pub const LEGACY_JVM: [&'static str; 3] = [
        "-Djava.library.path=${natives_directory}",
        "-cp",
        "${classpath}",
    ];

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mojang::rules::MojangOs;
    use serde_json::json;

    fn ctx(os: MojangOs, arch: &str) -> RuntimeContext {
        RuntimeContext {
            os,
            arch: crate::mojang::rules::normalize_arch(arch),
            os_version: "10.0".into(),
        }
    }

    fn libraries(value: serde_json::Value) -> Vec<Library> {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn modern_natives_are_plain_artifacts_filtered_by_rules() {
        let libs = libraries(json!([
            {
                "name": "org.lwjgl:lwjgl:3.3.1",
                "downloads": { "artifact": { "path": "org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1.jar", "url": "u", "sha1": "s", "size": 1 } }
            },
            {
                "name": "org.lwjgl:lwjgl:3.3.1:natives-macos-arm64",
                "downloads": { "artifact": { "path": "lwjgl-natives-macos-arm64.jar", "url": "u", "sha1": "s", "size": 1 } },
                "rules": [{ "action": "allow", "os": { "name": "osx" } }]
            },
            {
                "name": "org.lwjgl:lwjgl:3.3.1:natives-windows",
                "downloads": { "artifact": { "path": "lwjgl-natives-windows.jar", "url": "u", "sha1": "s", "size": 1 } },
                "rules": [{ "action": "allow", "os": { "name": "windows" } }]
            }
        ]));

        let on_mac = resolve_libraries(&libs, &ctx(MojangOs::Osx, "aarch64"));
        assert_eq!(on_mac.len(), 2);
        assert_eq!(on_mac[1].name.as_deref(), Some("org.lwjgl:lwjgl:3.3.1:natives-macos-arm64"));

        let on_linux = resolve_libraries(&libs, &ctx(MojangOs::Linux, "x86_64"));
        assert_eq!(on_linux.len(), 1);
    }

    #[test]
    fn legacy_natives_substitute_arch_placeholder() {
        let libs = libraries(json!([{
            "name": "org.lwjgl.lwjgl:lwjgl-platform:2.9.4",
            "natives": { "windows": "natives-windows-${arch}", "osx": "natives-osx" },
            "downloads": { "classifiers": {
                "natives-windows-32": { "path": "w32.jar", "url": "u" },
                "natives-windows-64": { "path": "w64.jar", "url": "u" },
                "natives-osx": { "path": "m.jar", "url": "u" }
            }}
        }]));

        let x64 = resolve_libraries(&libs, &ctx(MojangOs::Windows, "amd64"));
        assert_eq!(x64[0].native.as_ref().unwrap().path, "w64.jar");
        assert!(x64[0].artifact.is_none(), "native-only библиотека не идёт в classpath");

        let x86 = resolve_libraries(&libs, &ctx(MojangOs::Windows, "x86"));
        assert_eq!(x86[0].native.as_ref().unwrap().path, "w32.jar");

        let mac = resolve_libraries(&libs, &ctx(MojangOs::Osx, "x86_64"));
        assert_eq!(mac[0].native.as_ref().unwrap().path, "m.jar");
    }

    #[test]
    fn empty_url_means_locally_produced_artifact() {
        let libs = libraries(json!([{
            "name": "net.minecraftforge:forge:1.20.1-47.2.0:client",
            "downloads": { "artifact": { "path": "forge-client.jar", "url": "" } }
        }]));

        let resolved = resolve_libraries(&libs, &ctx(MojangOs::Windows, "amd64"));
        assert!(resolved[0].artifact.as_ref().unwrap().url.is_none());
    }

    #[test]
    fn order_is_kept_and_unusable_entries_are_dropped() {
        let libs = libraries(json!([
            { "name": "g:a:1", "downloads": { "artifact": { "path": "a.jar", "url": "u" } } },
            { "name": "не координата" },
            { "name": "g:b:1", "downloads": { "artifact": { "path": "b.jar", "url": "u" } } }
        ]));

        let names: Vec<_> = resolve_libraries(&libs, &ctx(MojangOs::Linux, "x86_64"))
            .into_iter()
            .filter_map(|lib| lib.name)
            .collect();

        assert_eq!(names, vec!["g:a:1", "g:b:1"]);
    }

    #[test]
    fn legacy_forge_libraries_are_resolved_from_coordinates() {
        let libs = libraries(json!([
            { "name": "net.minecraftforge:forge:1.12.2-14.23.5.2859" },
            { "name": "org.ow2.asm:asm:5.0.3", "url": "https://maven.example/" }
        ]));

        let resolved = resolve_libraries(&libs, &ctx(MojangOs::Windows, "amd64"));

        let forge = resolved[0].artifact.as_ref().unwrap();
        assert_eq!(
            forge.path,
            "net/minecraftforge/forge/1.12.2-14.23.5.2859/forge-1.12.2-14.23.5.2859.jar"
        );
        assert!(forge.url.as_ref().unwrap().starts_with(DEFAULT_MAVEN));

        let asm = resolved[1].artifact.as_ref().unwrap();
        assert_eq!(
            asm.url.as_deref(),
            Some("https://maven.example/org/ow2/asm/asm/5.0.3/asm-5.0.3.jar")
        );
    }

    #[test]
    fn java_requirement_falls_back_to_minecraft_version() {
        assert_eq!(JavaRequirement::from_minecraft_version("1.12.2").major, Some(8));
        assert_eq!(JavaRequirement::from_minecraft_version("1.17.1").major, Some(16));
        assert_eq!(JavaRequirement::from_minecraft_version("1.18").major, Some(17));
        assert_eq!(JavaRequirement::from_minecraft_version("1.20.4").major, Some(17));
        assert_eq!(JavaRequirement::from_minecraft_version("1.20.6").major, Some(21));
        assert_eq!(JavaRequirement::from_minecraft_version("1.21.1").major, Some(21));

        let future = JavaRequirement::from_minecraft_version("25.0");
        assert_eq!(future.major, Some(21));
        assert!(future.at_least);
    }

    #[test]
    fn java_requirement_prefers_manifest_over_guess() {
        let package: VersionPackage = serde_json::from_value(json!({
            "id": "1.12.2",
            "javaVersion": { "component": "java-runtime-gamma", "majorVersion": 17 },
            "libraries": []
        }))
        .unwrap();

        let requirement = JavaRequirement::from_package(&package);
        assert_eq!(requirement.major, Some(17));
        assert_eq!(requirement.component.as_deref(), Some("java-runtime-gamma"));
    }
}
