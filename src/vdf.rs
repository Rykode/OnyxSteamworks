use std::path::{Path, PathBuf};

use crate::model::{Depot, Project};

#[derive(Debug, thiserror::Error)]
pub enum VdfError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Default)]
pub struct BuildOptions {
    pub preview: bool,
    pub local: Option<String>,
    pub branch: String,
    pub desc: String,
    /// Upload the build without setting any branch live.
    pub upload_only: bool,
}

fn escape(s: &str) -> String {
    s.replace(['\r', '\n'], " ").replace('"', "'")
}

fn fwd_path(s: &str) -> String {
    s.replace('\\', "/")
}

fn path_string(p: &Path) -> String {
    p.to_string_lossy().to_string()
}

fn pair(indent: usize, key: &str, raw_value: &str) -> String {
    let tabs = "\t".repeat(indent);
    format!("{tabs}\"{}\"\t\"{}\"\n", escape(key), escape(raw_value))
}

fn setlive(project: &Project, options: &BuildOptions) -> String {
    let chosen = options.branch.trim();
    let branch = if chosen.is_empty() {
        project.set_live_branch.trim()
    } else {
        chosen
    };
    if branch.is_empty() || branch.eq_ignore_ascii_case("default") {
        String::new()
    } else {
        branch.to_string()
    }
}

fn depot_filename(depot_id: &str) -> String {
    format!("depot_build_{depot_id}.vdf")
}

pub fn resolved_desc(project: &Project, options: &BuildOptions) -> String {
    let raw = if options.desc.trim().is_empty() {
        project.desc_template.as_str()
    } else {
        options.desc.as_str()
    };
    if !raw.contains('{') {
        return raw.to_string();
    }
    let branch = setlive(project, options);
    let vars = crate::features::template::resolve(project, &branch);
    crate::features::template::expand(raw, &vars)
}

fn gen_appbuild(project: &Project, options: &BuildOptions) -> String {
    let mut s = String::from("\"appbuild\"\n{\n");
    s.push_str(&pair(1, "appid", &project.app_id));
    s.push_str(&pair(1, "desc", &resolved_desc(project, options)));
    s.push_str(&pair(
        1,
        "buildoutput",
        &fwd_path(&path_string(&project.build_output_dir)),
    ));
    s.push_str(&pair(
        1,
        "contentroot",
        &fwd_path(&path_string(&project.content_root)),
    ));
    let live = if options.upload_only {
        String::new()
    } else {
        setlive(project, options)
    };
    s.push_str(&pair(1, "setlive", &live));
    s.push_str(&pair(1, "preview", if options.preview { "1" } else { "0" }));
    if let Some(local) = &options.local {
        s.push_str(&pair(1, "local", &fwd_path(local)));
    }
    s.push_str("\t\"depots\"\n\t{\n");
    for depot in &project.depots {
        s.push_str(&pair(2, &depot.depot_id, &depot_filename(&depot.depot_id)));
    }
    s.push_str("\t}\n}\n");
    s
}

fn relative_local(content_root: &Path, local_path: &Path) -> String {
    let norm = |c: &std::path::Component| c.as_os_str().to_string_lossy().to_lowercase();
    let root: Vec<String> = content_root.components().map(|c| norm(&c)).collect();
    let parts: Vec<std::path::Component> = local_path.components().collect();
    let under_root =
        parts.len() >= root.len() && root.iter().zip(&parts).all(|(r, p)| *r == norm(p));
    let rel: Vec<String> = if under_root {
        parts[root.len()..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect()
    } else {
        parts
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect()
    };
    let joined = rel.join("/");
    let trimmed = joined.trim_matches('/');
    if trimmed.is_empty() {
        "*".to_string()
    } else {
        format!("{trimmed}/*")
    }
}

fn gen_depotbuild(depot: &Depot, content_root: &Path) -> String {
    let mut s = String::from("\"DepotBuildConfig\"\n{\n");
    s.push_str(&pair(1, "DepotID", &depot.depot_id));
    s.push_str("\t\"FileMapping\"\n\t{\n");
    s.push_str(&pair(
        2,
        "LocalPath",
        &relative_local(content_root, &depot.local_path),
    ));
    s.push_str(&pair(2, "DepotPath", &fwd_path(&depot.depot_path)));
    s.push_str(&pair(
        2,
        "recursive",
        if depot.recursive { "1" } else { "0" },
    ));
    s.push_str("\t}\n");
    for ex in &depot.exclusions {
        if !ex.trim().is_empty() {
            s.push_str(&pair(1, "FileExclusion", &fwd_path(ex.trim())));
        }
    }
    s.push_str("}\n");
    s
}

pub fn generate_vdfs(project: &Project, options: &BuildOptions) -> Vec<(String, String)> {
    let project = &crate::features::maintenance::resolve(project);
    let mut out = vec![("appbuild.vdf".to_string(), gen_appbuild(project, options))];
    for depot in &project.depots {
        out.push((
            depot_filename(&depot.depot_id),
            gen_depotbuild(depot, &project.content_root),
        ));
    }
    out
}

pub fn write_vdfs(
    project: &Project,
    options: &BuildOptions,
    work_root: &Path,
) -> Result<Vec<PathBuf>, VdfError> {
    let project_dir = work_root.join(&project.id);
    prune_runs(&project_dir, KEEP_RUNS);
    let dir = project_dir.join(format!("run-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir)?;
    let mut paths = Vec::new();
    for (name, content) in generate_vdfs(project, options) {
        let path = dir.join(&name);
        std::fs::write(&path, content)?;
        paths.push(path);
    }
    Ok(paths)
}

const KEEP_RUNS: usize = 10;

fn prune_runs(project_dir: &Path, keep: usize) {
    let Ok(entries) = std::fs::read_dir(project_dir) else {
        return;
    };
    let mut runs: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("run-"))
        .filter_map(|e| {
            let modified = e.metadata().and_then(|m| m.modified()).ok()?;
            Some((modified, e.path()))
        })
        .collect();
    if runs.len() < keep {
        return;
    }
    runs.sort_by_key(|r| std::cmp::Reverse(r.0));
    for (_, path) in runs.into_iter().skip(keep.saturating_sub(1)) {
        let _ = std::fs::remove_dir_all(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Depot;

    fn sample() -> Project {
        Project {
            app_id: "480".to_string(),
            desc_template: "Nightly".to_string(),
            content_root: "C:/game".into(),
            build_output_dir: "C:/out".into(),
            set_live_branch: "beta".to_string(),
            depots: vec![Depot {
                depot_id: "481".to_string(),
                local_path: "C:/game/win".into(),
                depot_path: ".".to_string(),
                recursive: true,
                exclusions: vec!["*.pdb".to_string(), "*.log".to_string()],
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn appbuild_exact() {
        let files = generate_vdfs(&sample(), &BuildOptions::default());
        let expected = "\"appbuild\"\n{\n\t\"appid\"\t\"480\"\n\t\"desc\"\t\"Nightly\"\n\t\"buildoutput\"\t\"C:/out\"\n\t\"contentroot\"\t\"C:/game\"\n\t\"setlive\"\t\"beta\"\n\t\"preview\"\t\"0\"\n\t\"depots\"\n\t{\n\t\t\"481\"\t\"depot_build_481.vdf\"\n\t}\n}\n";
        assert_eq!(files[0].0, "appbuild.vdf");
        assert_eq!(files[0].1, expected);
    }

    #[test]
    fn depotbuild_exact() {
        let files = generate_vdfs(&sample(), &BuildOptions::default());
        let expected = "\"DepotBuildConfig\"\n{\n\t\"DepotID\"\t\"481\"\n\t\"FileMapping\"\n\t{\n\t\t\"LocalPath\"\t\"win/*\"\n\t\t\"DepotPath\"\t\".\"\n\t\t\"recursive\"\t\"1\"\n\t}\n\t\"FileExclusion\"\t\"*.pdb\"\n\t\"FileExclusion\"\t\"*.log\"\n}\n";
        assert_eq!(files[1].0, "depot_build_481.vdf");
        assert_eq!(files[1].1, expected);
    }

    #[test]
    fn local_path_is_relative_to_content_root() {
        let mut p = sample();
        p.content_root = "D:/Storage/content".into();
        p.depots[0].local_path = "D:/Storage/content/windows".into();
        let out = generate_vdfs(&p, &BuildOptions::default());
        assert!(
            out[1].1.contains("\"LocalPath\"\t\"windows/*\"\n"),
            "got: {}",
            out[1].1
        );
    }

    #[test]
    fn local_path_equal_to_content_root_is_wildcard() {
        let mut p = sample();
        p.content_root = "D:/game".into();
        p.depots[0].local_path = "D:/game".into();
        let out = generate_vdfs(&p, &BuildOptions::default());
        assert!(
            out[1].1.contains("\"LocalPath\"\t\"*\"\n"),
            "got: {}",
            out[1].1
        );
    }

    #[test]
    fn local_path_matches_content_root_case_insensitively() {
        let mut p = sample();
        p.content_root = "D:/Storage/Content".into();
        p.depots[0].local_path = "d:/storage/content/linux".into();
        let out = generate_vdfs(&p, &BuildOptions::default());
        assert!(
            out[1].1.contains("\"LocalPath\"\t\"linux/*\"\n"),
            "got: {}",
            out[1].1
        );
    }

    #[test]
    fn setlive_empty_for_default_and_blank() {
        let mut p = sample();
        p.set_live_branch = "default".to_string();
        assert!(
            generate_vdfs(&p, &BuildOptions::default())[0]
                .1
                .contains("\"setlive\"\t\"\"\n")
        );
        p.set_live_branch = String::new();
        assert!(
            generate_vdfs(&p, &BuildOptions::default())[0]
                .1
                .contains("\"setlive\"\t\"\"\n")
        );
    }

    #[test]
    fn preview_and_local() {
        let opt = BuildOptions {
            preview: true,
            local: Some("D:/local".to_string()),
            ..Default::default()
        };
        let out = generate_vdfs(&sample(), &opt);
        assert!(out[0].1.contains("\"preview\"\t\"1\"\n"));
        assert!(out[0].1.contains("\"local\"\t\"D:/local\"\n"));
    }

    #[test]
    fn write_roundtrip() {
        let dir = std::env::temp_dir().join(format!("vdf-test-{}", uuid::Uuid::new_v4()));
        let mut p = sample();
        p.id = "pid".to_string();
        let paths = write_vdfs(&p, &BuildOptions::default(), &dir).expect("write");
        assert_eq!(paths.len(), 2);
        assert!(paths[0].ends_with("appbuild.vdf"));
        assert!(paths[0].starts_with(dir.join("pid")));
        let again = write_vdfs(&p, &BuildOptions::default(), &dir).expect("write");
        assert_ne!(paths[0].parent(), again[0].parent());
        assert!(paths[0].exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn old_runs_are_pruned() {
        let dir = std::env::temp_dir().join(format!("vdf-prune-{}", uuid::Uuid::new_v4()));
        let mut p = sample();
        p.id = "pid".to_string();
        for _ in 0..(KEEP_RUNS + 3) {
            write_vdfs(&p, &BuildOptions::default(), &dir).expect("write");
        }
        let count = std::fs::read_dir(dir.join("pid")).expect("dir").count();
        assert_eq!(count, KEEP_RUNS);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn upload_only_leaves_setlive_empty() {
        let opt = BuildOptions {
            branch: "staging".to_string(),
            upload_only: true,
            ..Default::default()
        };
        let out = generate_vdfs(&sample(), &opt);
        assert!(out[0].1.contains("\"setlive\"	\"\"
"), "{}", out[0].1);
    }

    #[test]
    fn chosen_branch_overrides_project_setlive() {
        let opt = BuildOptions {
            branch: "staging".to_string(),
            ..Default::default()
        };
        let out = generate_vdfs(&sample(), &opt);
        assert!(out[0].1.contains("\"setlive\"\t\"staging\"\n"));
    }

    #[test]
    fn desc_template_is_expanded_and_override_wins() {
        let mut p = sample();
        p.desc_template = "{branch} build".to_string();
        let out = generate_vdfs(&p, &BuildOptions::default());
        assert!(
            out[0].1.contains("\"desc\"\t\"beta build\"\n"),
            "{}",
            out[0].1
        );
        let opt = BuildOptions {
            desc: "hotfix for {branch}".to_string(),
            branch: "staging".to_string(),
            ..Default::default()
        };
        let out = generate_vdfs(&p, &opt);
        assert!(
            out[0].1.contains("\"desc\"\t\"hotfix for staging\"\n"),
            "{}",
            out[0].1
        );
    }

    #[test]
    fn free_text_cannot_break_out_of_quotes() {
        let mut p = sample();
        p.desc_template = "say \"hi\"\r\nnext C:\\dir\\".to_string();
        let out = generate_vdfs(&p, &BuildOptions::default());
        assert!(
            out[0]
                .1
                .contains("\"desc\"\t\"say 'hi'  next C:\\dir\\\"\n"),
            "{}",
            out[0].1
        );
    }

    #[test]
    fn exclusions_use_forward_slashes() {
        let mut p = sample();
        p.depots[0].exclusions = vec!["bin\\*.pdb".to_string()];
        let out = generate_vdfs(&p, &BuildOptions::default());
        assert!(
            out[1].1.contains("\"FileExclusion\"\t\"bin/*.pdb\"\n"),
            "{}",
            out[1].1
        );
    }
}
