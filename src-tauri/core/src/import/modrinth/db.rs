use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};

use crate::error::{CommandError, CommandResult};

pub const DB_FILE: &str = "app.db";

const SIDECARS: &[&str] = &["-wal"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Schema {
    Instances,
    Profiles,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstanceRow {
    pub path: String,
    pub name: String,
    pub icon_path: Option<String>,
    pub game_version: String,
    pub loader: String,
    pub loader_version: Option<String>,
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    pub java_path: Option<String>,
    pub memory_max: Option<u32>,
    pub last_played: Option<i64>,
    pub time_played: u64,
}

#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub custom_dir: Option<PathBuf>,
    pub instances: Vec<InstanceRow>,
}

pub fn read(settings_dir: &Path) -> CommandResult<Snapshot> {
    let source = settings_dir.join(DB_FILE);

    if !source.is_file() {
        return Err(CommandError::fs(format!(
            "В каталоге нет базы Modrinth App ({DB_FILE}): {}",
            settings_dir.display()
        )));
    }

    let scratch = Scratch::of(&source)?;
    let connection = open(&scratch.db)?;

    let snapshot = Snapshot {
        custom_dir: custom_dir(&connection),
        instances: instances(&connection)?,
    };

    Ok(snapshot)
}

fn open(path: &Path) -> CommandResult<Connection> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| CommandError::fs(format!("Не удалось открыть базу Modrinth App: {e}")))
}

pub fn schema(connection: &Connection) -> CommandResult<Schema> {
    let has = |name: &str| -> bool {
        connection
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [name],
                |row| row.get::<_, i64>(0),
            )
            .is_ok()
    };

    if has("instances") {
        return Ok(Schema::Instances);
    }

    if has("profiles") {
        return Ok(Schema::Profiles);
    }

    Err(CommandError::manifest(
        "База Modrinth App незнакомой версии: в ней нет ни instances, ни profiles",
    ))
}

fn custom_dir(connection: &Connection) -> Option<PathBuf> {
    connection
        .query_row("SELECT custom_dir FROM settings WHERE id = 0", [], |row| {
            row.get::<_, Option<String>>(0)
        })
        .ok()
        .flatten()
        .map(|dir| dir.trim().to_string())
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
}

const INSTANCES_QUERY: &str = "
    SELECT
        i.path,
        i.name,
        i.icon_path,
        cs.game_version,
        cs.loader,
        cs.loader_version,
        link.modrinth_project_id,
        link.modrinth_version_id,
        json_extract(json(ovr.overrides), '$.java_path'),
        json_extract(json(ovr.overrides), '$.memory.maximum'),
        i.last_played,
        i.submitted_time_played + i.recent_time_played
    FROM instances i
    LEFT JOIN instance_content_sets cs
        ON cs.instance_id = i.id
        AND cs.id = COALESCE(i.applied_content_set_id, (
            SELECT id FROM instance_content_sets
            WHERE instance_id = i.id
            ORDER BY created
            LIMIT 1
        ))
    LEFT JOIN instance_links link ON link.instance_id = i.id
    LEFT JOIN instance_launch_overrides ovr ON ovr.instance_id = i.id
";

const PROFILES_QUERY: &str = "
    SELECT
        path,
        name,
        icon_path,
        game_version,
        mod_loader,
        mod_loader_version,
        linked_project_id,
        linked_version_id,
        override_java_path,
        override_mc_memory_max,
        last_played,
        submitted_time_played + recent_time_played
    FROM profiles
";

fn instances(connection: &Connection) -> CommandResult<Vec<InstanceRow>> {
    let query = match schema(connection)? {
        Schema::Instances => INSTANCES_QUERY,
        Schema::Profiles => PROFILES_QUERY,
    };

    let failed = |e: rusqlite::Error| {
        CommandError::manifest(format!("Не удалось прочитать сборки Modrinth App: {e}"))
    };

    let mut statement = connection.prepare(query).map_err(failed)?;

    let rows = statement
        .query_map([], |row| {
            Ok(InstanceRow {
                path: row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                name: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                icon_path: text(row.get(2)?),
                game_version: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                loader: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                loader_version: text(row.get(5)?),
                project_id: text(row.get(6)?),
                version_id: text(row.get(7)?),
                java_path: text(row.get(8)?),
                memory_max: row.get::<_, Option<i64>>(9)?.and_then(|max| u32::try_from(max).ok()),
                last_played: row.get::<_, Option<i64>>(10)?,
                time_played: row.get::<_, Option<i64>>(11)?.unwrap_or(0).max(0) as u64,
            })
        })
        .map_err(failed)?;

    let mut found = Vec::new();

    for row in rows {
        found.push(row.map_err(failed)?);
    }

    Ok(found)
}

fn text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

struct Scratch {
    dir: PathBuf,
    db: PathBuf,
}

impl Scratch {
    fn of(source: &Path) -> CommandResult<Self> {
        let dir = std::env::temp_dir().join(format!("cast-modrinth-{}", uuid::Uuid::new_v4()));

        std::fs::create_dir_all(&dir)
            .map_err(|e| CommandError::io("Не удалось создать временный каталог", &dir, e))?;

        let copy = Self {
            db: dir.join(DB_FILE),
            dir,
        };

        std::fs::copy(source, &copy.db)
            .map_err(|e| CommandError::io("Не удалось прочитать базу Modrinth App", source, e))?;

        for suffix in SIDECARS {
            let sidecar = with_suffix(source, suffix);

            if sidecar.is_file() {
                std::fs::copy(&sidecar, with_suffix(&copy.db, suffix)).map_err(|e| {
                    CommandError::io("Не удалось прочитать журнал Modrinth App", &sidecar, e)
                })?;
            }
        }

        Ok(copy)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);

    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn database(schema: Schema) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cast-modrinth-db-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        let connection = Connection::open(dir.join(DB_FILE)).unwrap();

        connection
            .execute_batch(
                "CREATE TABLE settings (id INTEGER PRIMARY KEY, custom_dir TEXT NULL);
                 INSERT INTO settings (id, custom_dir) VALUES (0, NULL);",
            )
            .unwrap();

        match schema {
            Schema::Profiles => connection
                .execute_batch(
                    "CREATE TABLE profiles (
                        path TEXT PRIMARY KEY, name TEXT, icon_path TEXT,
                        game_version TEXT, mod_loader TEXT, mod_loader_version TEXT,
                        linked_project_id TEXT, linked_version_id TEXT,
                        override_java_path TEXT, override_mc_memory_max INTEGER,
                        last_played INTEGER,
                        submitted_time_played INTEGER DEFAULT 0,
                        recent_time_played INTEGER DEFAULT 0
                     );
                     INSERT INTO profiles VALUES (
                        'Fabulously Optimized', 'Fabulously Optimized', 'icons/fo.png',
                        '1.21.1', 'fabric', '0.16.5',
                        '1KVo5zza', 'fzpQA5K4',
                        'C:/jdk21/bin/javaw.exe', 6144,
                        1761212747, 700000, 5341
                     );",
                )
                .unwrap(),
            Schema::Instances => connection
                .execute_batch(
                    "CREATE TABLE instances (
                        id TEXT PRIMARY KEY, path TEXT, applied_content_set_id TEXT,
                        name TEXT, icon_path TEXT, last_played INTEGER,
                        submitted_time_played INTEGER DEFAULT 0,
                        recent_time_played INTEGER DEFAULT 0
                     );
                     CREATE TABLE instance_content_sets (
                        id TEXT PRIMARY KEY, instance_id TEXT, created INTEGER,
                        game_version TEXT, loader TEXT, loader_version TEXT
                     );
                     CREATE TABLE instance_links (
                        instance_id TEXT PRIMARY KEY,
                        modrinth_project_id TEXT, modrinth_version_id TEXT
                     );
                     CREATE TABLE instance_launch_overrides (
                        instance_id TEXT PRIMARY KEY, overrides BLOB
                     );
                     INSERT INTO instances VALUES (
                        'inst-1', 'Fabulously Optimized', 'set-1',
                        'Fabulously Optimized', 'icons/fo.png', 1761212747, 700000, 5341
                     );
                     INSERT INTO instance_content_sets VALUES (
                        'set-1', 'inst-1', 1, '1.21.1', 'fabric', '0.16.5'
                     );
                     INSERT INTO instance_links VALUES ('inst-1', '1KVo5zza', 'fzpQA5K4');
                     INSERT INTO instance_launch_overrides VALUES (
                        'inst-1',
                        jsonb(json_object(
                            'java_path', 'C:/jdk21/bin/javaw.exe',
                            'memory', json_object('maximum', 6144)
                        ))
                     );",
                )
                .unwrap(),
        }

        drop(connection);

        dir
    }

    fn only(dir: &Path) -> InstanceRow {
        let mut snapshot = read(dir).unwrap();

        assert_eq!(snapshot.instances.len(), 1);
        snapshot.instances.pop().unwrap()
    }

    #[test]
    fn the_new_schema_is_read_through_its_joins() {
        let dir = database(Schema::Instances);
        let row = only(&dir);

        assert_eq!(row.path, "Fabulously Optimized");
        assert_eq!(row.game_version, "1.21.1");
        assert_eq!(row.loader, "fabric");
        assert_eq!(row.loader_version.as_deref(), Some("0.16.5"));
        assert_eq!(row.project_id.as_deref(), Some("1KVo5zza"));
        assert_eq!(row.version_id.as_deref(), Some("fzpQA5K4"));
        assert_eq!(row.java_path.as_deref(), Some("C:/jdk21/bin/javaw.exe"));
        assert_eq!(row.memory_max, Some(6144));
        assert_eq!(row.last_played, Some(1_761_212_747));
        assert_eq!(row.time_played, 705_341);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_old_schema_yields_exactly_the_same_row() {
        let new_dir = database(Schema::Instances);
        let old_dir = database(Schema::Profiles);

        assert_eq!(only(&new_dir), only(&old_dir));

        std::fs::remove_dir_all(&new_dir).ok();
        std::fs::remove_dir_all(&old_dir).ok();
    }

    #[test]
    fn a_custom_directory_moves_the_content_elsewhere() {
        let dir = database(Schema::Instances);

        let connection = Connection::open(dir.join(DB_FILE)).unwrap();
        connection
            .execute("UPDATE settings SET custom_dir = 'D:/Games/Modrinth'", [])
            .unwrap();
        drop(connection);

        assert_eq!(
            read(&dir).unwrap().custom_dir,
            Some(PathBuf::from("D:/Games/Modrinth"))
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_original_database_is_never_touched() {
        let dir = database(Schema::Instances);
        let before = std::fs::metadata(dir.join(DB_FILE)).unwrap().len();

        read(&dir).unwrap();

        assert_eq!(std::fs::metadata(dir.join(DB_FILE)).unwrap().len(), before);
        assert!(!dir.join(format!("{DB_FILE}-wal")).exists(), "журнал не появился");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_tail_left_in_the_journal_is_still_seen() {
        let dir = database(Schema::Instances);

        let connection = Connection::open(dir.join(DB_FILE)).unwrap();
        connection.pragma_update(None, "journal_mode", "WAL").unwrap();
        connection
            .execute("UPDATE instances SET name = 'Переименовали'", [])
            .unwrap();

        assert!(dir.join(format!("{DB_FILE}-wal")).is_file(), "хвост правда в журнале");

        let name = only(&dir).name;
        drop(connection);

        assert_eq!(name, "Переименовали", "журнал восстановлен без -shm");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_database_without_familiar_tables_is_refused() {
        let dir = std::env::temp_dir().join(format!("cast-modrinth-db-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        let connection = Connection::open(dir.join(DB_FILE)).unwrap();
        connection.execute("CREATE TABLE bookmarks (x INTEGER)", []).unwrap();
        drop(connection);

        let error = read(&dir).unwrap_err();
        assert!(error.message.contains("instances"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_missing_database_says_so_plainly() {
        let dir = std::env::temp_dir().join(format!("cast-modrinth-db-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        assert!(read(&dir).unwrap_err().message.contains(DB_FILE));

        std::fs::remove_dir_all(&dir).ok();
    }
}
