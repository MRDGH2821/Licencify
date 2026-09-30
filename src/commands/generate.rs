use crate::{
    cli::LicenseFormat, fs::global_fs, licence_name::LicenceName, resolution::ResolvedContext,
    template,
};
use anyhow::Context;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};

/// Primary licence text and the path both add and update write.
pub struct RenderedPrimary {
    pub path: PathBuf,
    pub content: String,
}

/// Replacement prepared in memory. Nothing on disk has changed yet.
pub struct PlannedPrimary {
    pub path: PathBuf,
    pub content: String,
    pub format: String,
    pub template_source: String,
    pub previous: Option<PathBuf>,
    pub promotion_source: Option<PathBuf>,
    /// Set only when a promotion must replace the additional-licence list.
    pub additional_after: Option<Vec<String>>,
}

/// Render the primary licence. Add and update share this path so format and
/// placeholder decisions cannot drift between commands.
pub fn render_primary(ctx: &ResolvedContext) -> anyhow::Result<RenderedPrimary> {
    let render_ctx = template::render_context(
        &ctx.year,
        &ctx.author,
        ctx.company.as_deref(),
        ctx.email.as_deref(),
    );
    let ext = ctx.resolved.format.to_string();
    let content = match &ctx.resolved.format {
        LicenseFormat::Md => {
            template::render_markdown_with_context(&ctx.resolved.text, &render_ctx)?
        }
        _ => template::render_with_context(&ctx.resolved.text, &render_ctx)?,
    };
    let path = ctx.licence_name.file_path(&ext);
    ensure_output_path(&path)?;
    Ok(RenderedPrimary { path, content })
}

/// Choose the SPDX id. Update must receive one; add may use the effective setting.
pub fn selected_licence_id(
    explicit: Option<&str>,
    configured: Option<&str>,
    require_explicit: bool,
) -> anyhow::Result<String> {
    if let Some(id) = explicit.map(str::trim).filter(|id| !id.is_empty()) {
        return Ok(id.to_string());
    }
    if require_explicit {
        anyhow::bail!("update requires a replacement licence ID");
    }
    configured
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .context("No licence specified. Pass an SPDX ID or set `licence` in configuration.")
}

/// Resolve the file transition. Returns before any licence file is created or removed.
pub fn plan_primary(
    ctx: &ResolvedContext,
    requested_id: &str,
    canonical_id: &str,
    additional: &[String],
    permit_promotion: bool,
    require_existing: bool,
) -> anyhow::Result<PlannedPrimary> {
    let rendered = render_primary(ctx)?;
    let fs = global_fs();
    let primaries: Vec<PathBuf> = LicenceName::primary_variants()
        .into_iter()
        .filter(|path| fs.exists(path))
        .collect();
    if primaries.len() > 1 {
        anyhow::bail!(
            "Multiple primary licence files ({}); not changing any files",
            primaries
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if require_existing && primaries.is_empty() {
        anyhow::bail!("No primary licence file found. Use `licencify add` to create one first.");
    }

    let mut path = rendered.path;
    let mut content = rendered.content;
    let mut format = ctx.resolved.format.to_string();
    let mut promotion_source = None;
    let mut additional_after = None;
    if is_additional(additional, requested_id, canonical_id) {
        if !permit_promotion {
            anyhow::bail!(
                "{canonical_id} is listed in additional-licences. Pass --permit-promotion to promote it. --yes does not authorize promotion."
            );
        }
        let extras = existing_extras(&*fs, requested_id, canonical_id, additional);
        if extras.len() != 1 {
            anyhow::bail!(
                "Promotion of {canonical_id} needs exactly one matching additional file; found {}. Not changing any files",
                extras.len()
            );
        }
        let source = extras.into_iter().next().expect("length checked");
        let ext = source
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        anyhow::ensure!(
            matches!(ext.as_str(), "txt" | "html" | "md"),
            "Invalid additional licence output path {}",
            source.display()
        );
        content = fs
            .read_to_string(&source)
            .with_context(|| format!("Could not read additional licence {}", source.display()))?;
        path = ctx.licence_name.file_path(&ext);
        ensure_output_path(&path)?;
        format = ext;
        promotion_source = Some(source);
        additional_after = Some(
            additional
                .iter()
                .filter(|id| {
                    !id.eq_ignore_ascii_case(requested_id) && !id.eq_ignore_ascii_case(canonical_id)
                })
                .cloned()
                .collect(),
        );
    }

    Ok(PlannedPrimary {
        path,
        content,
        format,
        template_source: ctx.resolved.source.clone(),
        previous: primaries.into_iter().next(),
        promotion_source,
        additional_after,
    })
}

/// Ask before mutating. `--yes` and a non-interactive stdin skip the prompt.
/// An empty answer accepts when `default_yes` is set, and declines otherwise.
pub fn confirm_proceed(yes: bool, prompt: &str, default_yes: bool) -> anyhow::Result<bool> {
    if yes || !std::io::stdin().is_terminal() {
        return Ok(true);
    }
    print!("{prompt}");
    std::io::stdout().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let answer = input.trim();
    if answer.is_empty() {
        return Ok(default_yes);
    }
    Ok(answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes"))
}

/// Stage the replacement, record config, then publish it. The previous primary
/// stays in place until the staged file is ready and config has been saved.
pub fn commit_primary(
    plan: &PlannedPrimary,
    license_id: &str,
    author: &str,
) -> anyhow::Result<bool> {
    let fs = global_fs();
    let stage = staging_path(&plan.path);
    anyhow::ensure!(
        !fs.exists(&stage),
        "Refusing to overwrite staging file {}",
        stage.display()
    );
    fs.write(&stage, &plan.content)
        .with_context(|| format!("Failed to stage replacement {}", stage.display()))?;

    let saved = match crate::config::Config::update_saved_selection(
        license_id,
        author,
        &plan.format,
        plan.additional_after.as_deref(),
    ) {
        Ok(saved) => saved,
        Err(error) => {
            return Err(error.context(
                "Replacement was staged and the existing primary was left unchanged because configuration could not be updated",
            ));
        }
    };

    if let Err(error) = fs.rename(&stage, &plan.path) {
        return Err(anyhow::Error::from(error).context(format!(
            "Replacement remains staged at {} and the existing primary was left in place",
            stage.display()
        )));
    }

    if let Some(previous) = &plan.previous {
        if previous != &plan.path {
            if let Err(error) = fs.remove_file(previous) {
                return Err(anyhow::Error::from(error).context(format!(
                    "Replacement is at {} and the previous primary {} was left in place",
                    plan.path.display(),
                    previous.display()
                )));
            }
        }
    }

    if let Some(source) = &plan.promotion_source {
        if source != &plan.path && fs.exists(source) {
            if let Err(error) = fs.remove_file(source) {
                return Err(anyhow::Error::from(error).context(format!(
                    "Promoted primary is at {} and additional file {} was left in place",
                    plan.path.display(),
                    source.display()
                )));
            }
        }
    }
    Ok(saved)
}

fn ensure_output_path(path: &Path) -> anyhow::Result<()> {
    anyhow::ensure!(
        path.components().count() == 1
            && path
                .file_name()
                .is_some_and(|name| !name.is_empty() && name != "." && name != ".."),
        "Invalid licence output path {}",
        path.display()
    );
    Ok(())
}

fn staging_path(dest: &Path) -> PathBuf {
    let mut name = dest.as_os_str().to_os_string();
    name.push(".licencify-new");
    PathBuf::from(name)
}

fn is_additional(additional: &[String], requested_id: &str, canonical_id: &str) -> bool {
    additional
        .iter()
        .any(|id| id.eq_ignore_ascii_case(requested_id) || id.eq_ignore_ascii_case(canonical_id))
}

fn existing_extras(
    fs: &dyn crate::fs::Fs,
    requested_id: &str,
    canonical_id: &str,
    additional: &[String],
) -> Vec<PathBuf> {
    let mut ids = vec![requested_id.to_string(), canonical_id.to_string()];
    for id in additional {
        if id.eq_ignore_ascii_case(requested_id) || id.eq_ignore_ascii_case(canonical_id) {
            ids.push(id.clone());
        }
    }
    let mut found = Vec::new();
    for id in ids {
        for path in LicenceName::extra_variants(&id) {
            if fs.exists(&path) && !found.iter().any(|existing| existing == &path) {
                found.push(path);
            }
        }
    }
    found
}
