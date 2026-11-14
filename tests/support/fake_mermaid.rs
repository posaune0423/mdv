use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub fn successful_command(
    directory: &Path,
    png_path: &Path,
    args_path: Option<&Path>,
    counter_path: Option<&Path>,
) -> io::Result<PathBuf> {
    let command = command_path(directory);

    #[cfg(windows)]
    {
        fs::write(&command, windows_success_script(png_path, args_path, counter_path))?;
        make_executable(&command)?;
    }

    #[cfg(not(windows))]
    {
        fs::write(
            dynamic_content_path(directory),
            unix_success_script(png_path, args_path, counter_path),
        )?;
        link_launcher(&command)?;
    }
    Ok(command)
}

#[allow(dead_code)]
pub fn failing_command(directory: &Path) -> io::Result<PathBuf> {
    let command = command_path(directory);
    #[cfg(windows)]
    {
        fs::write(&command, "@echo off\r\n>&2 echo parse error near node A\r\nexit /b 7\r\n")?;
        make_executable(&command)?;
    }

    #[cfg(not(windows))]
    {
        fs::write(
            dynamic_content_path(directory),
            "#!/bin/sh\necho 'parse error near node A' >&2\nexit 7\n",
        )?;
        link_launcher(&command)?;
    }
    Ok(command)
}

#[cfg(not(windows))]
pub fn dynamic_content_path(directory: &Path) -> PathBuf {
    directory.join("fake mmdc-body.sh")
}

#[cfg(windows)]
fn command_path(directory: &Path) -> PathBuf {
    directory.join("fake mmdc.cmd")
}

#[cfg(not(windows))]
fn command_path(directory: &Path) -> PathBuf {
    directory.join("fake mmdc.sh")
}

#[cfg(windows)]
fn windows_success_script(
    png_path: &Path,
    args_path: Option<&Path>,
    counter_path: Option<&Path>,
) -> String {
    let args_setup = args_path
        .map_or_else(String::new, |path| format!("set \"args_file={}\"\r\n", path.display()));
    let counter_setup = counter_path.map_or_else(String::new, |path| {
        format!(
            "set \"counter_file={}\"\r\nset \"count=0\"\r\nif exist \"%counter_file%\" set /p count=<\"%counter_file%\"\r\nset /a count+=1\r\n<nul set /p \"=%count%\">\"%counter_file%\"\r\n",
            path.display()
        )
    });

    format!(
        "@echo off\r\nsetlocal EnableExtensions DisableDelayedExpansion\r\nset \"out=\"\r\n{args_setup}{counter_setup}:arguments\r\nif \"%~1\"==\"\" goto done\r\nif defined args_file >>\"%args_file%\" echo(%~1\r\nif \"%~1\"==\"-o\" set \"out=%~2\"\r\nshift\r\ngoto arguments\r\n:done\r\ncopy /Y \"{}\" \"%out%\" >NUL\r\n",
        png_path.display()
    )
}

#[cfg(not(windows))]
fn unix_success_script(
    png_path: &Path,
    args_path: Option<&Path>,
    counter_path: Option<&Path>,
) -> String {
    let args_setup =
        args_path.map_or_else(String::new, |path| format!("args_file={}\n", shell_quote(path)));
    let counter_setup = counter_path.map_or_else(String::new, |path| {
        let path = shell_quote(path);
        format!(
            "counter_file={path}\ncount=0\nif [ -f \"$counter_file\" ]; then count=$(cat \"$counter_file\"); fi\ncount=$((count + 1))\nprintf '%s' \"$count\" > \"$counter_file\"\n"
        )
    });

    format!(
        "#!/bin/sh\nset -eu\nout=\"\"\n{args_setup}{counter_setup}if [ -n \"${{args_file:-}}\" ]; then printf '%s\\n' \"$@\" > \"$args_file\"; fi\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = \"-o\" ]; then out=\"$2\"; fi\n  shift\ndone\ncp {} \"$out\"\n",
        shell_quote(png_path)
    )
}

#[cfg(not(windows))]
fn shell_quote(path: &Path) -> String {
    let path = path.to_string_lossy();
    format!("'{}'", path.replace('\'', "'\\\"'\\\"'"))
}

#[cfg(windows)]
fn make_executable(_command: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(not(windows))]
fn link_launcher(command: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/fake_mermaid_launcher.sh"),
        command,
    )
}
