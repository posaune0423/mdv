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
    let script = windows_success_script(png_path, args_path, counter_path);
    #[cfg(not(windows))]
    let script = unix_success_script(png_path, args_path, counter_path);

    fs::write(&command, script)?;
    make_executable(&command)?;
    Ok(command)
}

#[allow(dead_code)]
pub fn failing_command(directory: &Path) -> io::Result<PathBuf> {
    let command = command_path(directory);
    #[cfg(windows)]
    let script = "@echo off\r\n>&2 echo parse error near node A\r\nexit /b 7\r\n";
    #[cfg(not(windows))]
    let script = "#!/bin/sh\necho 'parse error near node A' >&2\nexit 7\n";

    fs::write(&command, script)?;
    make_executable(&command)?;
    Ok(command)
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
fn make_executable(command: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(command)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(command, permissions)
}
