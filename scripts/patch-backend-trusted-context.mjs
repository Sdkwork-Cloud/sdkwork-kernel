import fs from 'node:fs';
import path from 'node:path';

const file = path.join('sdkwork-agent-business', 'src', 'http.rs');
let source = fs.readFileSync(file, 'utf8');

source = source.replaceAll(
  'RequestScope::from_legacy_headers(',
  'RequestScope::from_trusted_extension(context, ',
);
source = source.replace(
  /RequestScope::from_trusted_extension\(context,([^)]+), headers\)/g,
  'RequestScope::from_trusted_extension(context,$1)',
);

const handlerNeedle =
  /async fn (backend_|open_|list_knowledge_|create_knowledge_|get_knowledge_|update_knowledge_|delete_knowledge_|restore_knowledge_|upsert_knowledge_|create_memory_|get_memory_|update_memory_|delete_memory_|restore_memory_|list_memory_|upsert_memory_)/;

const lines = source.split('\n');
const out = [];
let skipTestModule = false;

for (let i = 0; i < lines.length; i += 1) {
  const line = lines[i];

  if (line.includes('mod tests {') && line.includes('#[cfg(test)]')) {
    skipTestModule = true;
  }
  if (skipTestModule && line === '}' && out.at(-1)?.includes('}')) {
    skipTestModule = false;
  }

  if (line.trim() === 'headers: HeaderMap,' && !skipTestModule) {
    const window = out.slice(-12).join('\n');
    if (handlerNeedle.test(window)) {
      if (!window.includes('Extension(context): Extension<AgentRequestContext>')) {
        out.push('    Extension(context): Extension<AgentRequestContext>,');
      }
      continue;
    }
  }

  out.push(line);
}

let patched = out.join('\n');
patched = patched.replaceAll(
  'with_service_mut(&state, |',
  'with_service_mut(&state, move |',
);
patched = patched.replaceAll(
  'with_service_mut(&state, move |service| service.',
  'with_service_mut(&state, |service| service.',
);

fs.writeFileSync(file, patched);

const legacy = (patched.match(/from_legacy_headers\(/g) ?? []).length;
const trusted = (patched.match(/from_trusted_extension\(context,/g) ?? []).length;
const headers = (patched.match(/^\s+headers: HeaderMap,/gm) ?? []).length;
console.log({ legacy, trusted, headers });
