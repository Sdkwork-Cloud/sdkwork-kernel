export const WINDOWS_CARGO_FILESYSTEM_MAX_ATTEMPTS = 3;

export function isWindowsCargoFilesystemRace(output, platform = process.platform) {
  if (platform !== 'win32' || !output.includes('os error 5')) {
    return false;
  }
  return (
    output.includes('failed to link or copy') ||
    output.includes('failed to move dependency graph')
  );
}

export function shouldRetryWindowsCargoCommand(
  command,
  output,
  attempt,
  platform = process.platform
) {
  return (
    command === 'cargo' &&
    attempt < WINDOWS_CARGO_FILESYSTEM_MAX_ATTEMPTS &&
    isWindowsCargoFilesystemRace(output, platform)
  );
}
