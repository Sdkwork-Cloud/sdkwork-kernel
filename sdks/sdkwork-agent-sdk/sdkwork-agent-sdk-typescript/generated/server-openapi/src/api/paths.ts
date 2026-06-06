export const AGENT_API_PREFIX = '/agent/v3/api';

export function agentApiPath(path: string): string {
  if (!path) {
    return AGENT_API_PREFIX;
  }
  if (/^https?:\/\//i.test(path)) {
    return path;
  }
  const normalizedPrefixRaw = (AGENT_API_PREFIX || '').trim();
  const normalizedPrefix = normalizedPrefixRaw
    ? `/${normalizedPrefixRaw.replace(/^\/+|\/+$/g, '')}`
    : '';
  const normalizedPath = path.startsWith('/') ? path : `/${path}`;

  if (!normalizedPrefix || normalizedPrefix === '/') {
    return normalizedPath;
  }
  if (normalizedPath === normalizedPrefix || normalizedPath.startsWith(`${normalizedPrefix}/`)) {
    return normalizedPath;
  }
  return `${normalizedPrefix}${normalizedPath}`;
}
