use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

pub(super) fn write_document(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let path = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => fs::canonicalize(path)?,
        Ok(_) => path.to_owned(),
        Err(error) if error.kind() == ErrorKind::NotFound => path.to_owned(),
        Err(error) => return Err(error),
    };
    let permissions = match fs::metadata(&path) {
        Ok(metadata) => {
            if !metadata.is_file() {
                return Err(io::Error::new(
                    ErrorKind::InvalidInput,
                    "not a regular file",
                ));
            }
            if metadata.permissions().readonly() {
                return Err(io::Error::new(
                    ErrorKind::PermissionDenied,
                    "file is read-only",
                ));
            }
            OpenOptions::new().write(true).open(&path)?;
            Some(metadata.permissions())
        }
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;

    let (temporary, mut file) = loop {
        let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let temporary = parent.join(format!(".unikode-{}-{sequence}.tmp", std::process::id()));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(if permissions.is_some() { 0o600 } else { 0o666 });
        }
        match options.open(&temporary) {
            Ok(file) => break (temporary, file),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };
    let result = (|| {
        file.write_all(bytes)?;
        if let Some(permissions) = permissions {
            file.set_permissions(permissions)?;
        }
        file.sync_all()?;
        drop(file);
        // shortcut: preserve permissions only, revisit for hardlinks or extended metadata
        fs::rename(&temporary, &path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("unikode-save-test-{}-{id}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn creates_parent_directories_and_replaces_the_complete_document() {
        let directory = TestDirectory::new();
        let path = directory.0.join("nested/document");
        write_document(&path, b"first document").unwrap();
        write_document(&path, b"next").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"next");
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    }

    #[test]
    fn cleans_up_the_staged_file_if_rename_fails() {
        let directory = TestDirectory::new();
        let original = directory.0.join("document");
        fs::write(&original, b"original").unwrap();
        let invalid_target = directory.0.join("missing/..");
        assert!(write_document(&invalid_target, b"replacement").is_err());
        assert_eq!(fs::read(original).unwrap(), b"original");
        assert_eq!(
            fs::read_dir(directory.0.join("missing")).unwrap().count(),
            0
        );
    }

    #[test]
    fn accepts_a_bare_relative_filename() {
        let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let name = format!(".unikode-relative-test-{}-{id}", std::process::id());
        let path = Path::new(&name);
        write_document(path, b"relative").unwrap();
        let actual = fs::read(path).unwrap();
        fs::remove_file(path).unwrap();
        assert_eq!(actual, b"relative");
    }

    #[test]
    fn rejects_directories_without_changing_their_contents() {
        let directory = TestDirectory::new();
        let path = directory.0.join("document");
        fs::write(&path, b"original").unwrap();
        assert_eq!(
            write_document(&directory.0, b"replacement")
                .unwrap_err()
                .kind(),
            ErrorKind::InvalidInput
        );
        assert_eq!(fs::read(path).unwrap(), b"original");
        assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 1);
    }

    #[test]
    fn rejects_readonly_files_without_changing_them_or_leaving_temporary_files() {
        let directory = TestDirectory::new();
        let path = directory.0.join("document");
        fs::write(&path, b"original").unwrap();
        let original_permissions = fs::metadata(&path).unwrap().permissions();
        let mut readonly = original_permissions.clone();
        readonly.set_readonly(true);
        fs::set_permissions(&path, readonly).unwrap();
        let error = write_document(&path, b"replacement").unwrap_err();
        fs::set_permissions(&path, original_permissions).unwrap();
        assert_eq!(error.kind(), ErrorKind::PermissionDenied);
        assert_eq!(fs::read(path).unwrap(), b"original");
        assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn preserves_existing_unix_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let directory = TestDirectory::new();
        let path = directory.0.join("document");
        fs::write(&path, b"original").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        write_document(&path, b"replacement").unwrap();
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }

    #[cfg(unix)]
    #[test]
    fn replaces_the_symlink_target_without_replacing_the_link() {
        use std::os::unix::fs::symlink;
        let directory = TestDirectory::new();
        let target = directory.0.join("document");
        let link = directory.0.join("link");
        fs::write(&target, b"original").unwrap();
        symlink("document", &link).unwrap();
        write_document(&link, b"replacement").unwrap();
        assert_eq!(fs::read(target).unwrap(), b"replacement");
        assert_eq!(fs::read_link(link).unwrap(), Path::new("document"));
        assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_dangling_symlinks_without_replacing_them() {
        use std::os::unix::fs::symlink;
        let directory = TestDirectory::new();
        let link = directory.0.join("link");
        symlink("missing", &link).unwrap();
        assert_eq!(
            write_document(&link, b"replacement").unwrap_err().kind(),
            ErrorKind::NotFound
        );
        assert_eq!(fs::read_link(link).unwrap(), Path::new("missing"));
        assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 1);
    }
}
