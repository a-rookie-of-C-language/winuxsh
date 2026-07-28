use std::fs;

fn main() {
    println!("cargo:rerun-if-changed=Cargo.lock");

    let revision = fs::read_to_string("Cargo.lock")
        .ok()
        .and_then(|lock| rubash_revision_from_lock(&lock))
        .unwrap_or_else(|| "master".to_string());

    println!("cargo:rustc-env=WINUXSH_RUBASH_REV={revision}");
}

fn rubash_revision_from_lock(lock: &str) -> Option<String> {
    let mut in_rubash = false;

    for line in lock.lines() {
        let trimmed = line.trim();
        if trimmed == "[[package]]" {
            in_rubash = false;
            continue;
        }

        if trimmed == "name = \"rubash\"" {
            in_rubash = true;
            continue;
        }

        if in_rubash && trimmed.starts_with("source = ") {
            let source = trimmed.trim_start_matches("source = ").trim_matches('"');
            return source
                .rsplit_once('#')
                .map(|(_, rev)| rev.to_string())
                .or_else(|| Some("master".to_string()));
        }
    }

    None
}
