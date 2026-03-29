use comrak::Options;

pub(crate) fn gfm_options() -> Options<'static> {
    let mut options = Options::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    options.extension.alerts = true;
    options.render.tasklist_classes = true;
    options.render.github_pre_lang = true;
    options
}
