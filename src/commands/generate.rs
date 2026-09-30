use crate::{cli::LicenseFormat, fs::global_fs, resolution::ResolvedContext, template};
use std::path::PathBuf;

/// Primary licence text and the path both add and update write.
pub struct RenderedPrimary {
    pub path: PathBuf,
    pub content: String,
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
    Ok(RenderedPrimary {
        path: ctx.licence_name.file_path(&ext),
        content,
    })
}

/// Write a primary licence that `render_primary` already prepared.
pub fn write_primary(rendered: &RenderedPrimary) -> anyhow::Result<()> {
    global_fs().write(&rendered.path, &rendered.content)?;
    Ok(())
}
