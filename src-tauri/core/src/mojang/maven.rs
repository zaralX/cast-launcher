use crate::error::{CommandError, CommandResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gradle {
    pub group: String,
    pub artifact: String,
    pub version: String,
    pub classifier: Option<String>,
    pub extension: String,
}

impl Gradle {
    pub fn parse(coordinate: &str) -> CommandResult<Self> {
        let (body, extension) = match coordinate.split_once('@') {
            Some((body, extension)) if !extension.is_empty() => (body, extension.to_string()),
            _ => (coordinate, "jar".to_string()),
        };

        let mut parts = body.split(':');
        let group = parts.next().filter(|part| !part.is_empty());
        let artifact = parts.next().filter(|part| !part.is_empty());
        let version = parts.next().filter(|part| !part.is_empty());

        let (Some(group), Some(artifact), Some(version)) = (group, artifact, version) else {
            return Err(CommandError::manifest(format!(
                "Некорректная maven-координата: {coordinate}"
            )));
        };

        Ok(Self {
            group: group.to_string(),
            artifact: artifact.to_string(),
            version: version.to_string(),
            classifier: parts.next().filter(|part| !part.is_empty()).map(str::to_string),
            extension,
        })
    }

    pub fn path(&self) -> String {
        let group = self.group.replace('.', "/");
        let suffix = match &self.classifier {
            Some(classifier) => format!("-{classifier}"),
            None => String::new(),
        };

        format!(
            "{group}/{artifact}/{version}/{artifact}-{version}{suffix}.{extension}",
            artifact = self.artifact,
            version = self.version,
            extension = self.extension
        )
    }

    pub fn url(&self, repository: &str) -> String {
        format!("{}/{}", repository.trim_end_matches('/'), self.path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_coordinate() {
        let gradle = Gradle::parse("net.fabricmc:fabric-loader:0.15.7").unwrap();

        assert_eq!(gradle.path(), "net/fabricmc/fabric-loader/0.15.7/fabric-loader-0.15.7.jar");
        assert_eq!(
            gradle.url("https://maven.fabricmc.net/"),
            "https://maven.fabricmc.net/net/fabricmc/fabric-loader/0.15.7/fabric-loader-0.15.7.jar"
        );
    }

    #[test]
    fn classifier_and_extension() {
        let gradle = Gradle::parse("net.minecraftforge:forge:1.20.1-47.2.0:universal@zip").unwrap();

        assert_eq!(gradle.classifier.as_deref(), Some("universal"));
        assert_eq!(gradle.extension, "zip");
        assert_eq!(
            gradle.path(),
            "net/minecraftforge/forge/1.20.1-47.2.0/forge-1.20.1-47.2.0-universal.zip"
        );
    }

    #[test]
    fn incomplete_coordinates_are_rejected() {
        assert!(Gradle::parse("group:artifact").is_err());
        assert!(Gradle::parse("").is_err());
        assert!(Gradle::parse("group::1.0").is_err());
    }
}
