import fs from 'node:fs';
import path from 'node:path';

const file = path.join('sdkwork-agent-business', 'src', 'http.rs');
const lines = fs.readFileSync(file, 'utf8').split('\n');
const out = [];
let i = 0;

while (i < lines.length) {
  const line = lines[i];
  out.push(line);

  if (line.startsWith('async fn ') && !line.includes('inject_gateway')) {
    const fnLines = [line];
    let j = i + 1;
    while (j < lines.length && !lines[j].startsWith('async fn ') && lines[j] !== '}') {
      if (lines[j].startsWith(') ->')) {
        fnLines.push(lines[j]);
        j += 1;
        break;
      }
      fnLines.push(lines[j]);
      j += 1;
    }

    const signature = fnLines.join('\n');
    const usesTrusted = lines.slice(i, j + 40).some((l) => l.includes('from_trusted_extension(context'));

    if (usesTrusted && !signature.includes('Extension(context): Extension<AgentRequestContext>')) {
      const stateIdx = fnLines.findIndex((l) => l.includes('State(state): State<AgentHttpState>'));
      if (stateIdx >= 0) {
        out.pop();
        for (let k = 0; k < fnLines.length; k += 1) {
          out.push(fnLines[k]);
          if (k === stateIdx) {
            out.push('    Extension(context): Extension<AgentRequestContext>,');
          }
        }
        i = j;
        continue;
      }
    }

    for (let k = 1; k < fnLines.length; k += 1) {
      out.push(fnLines[k]);
    }
    i = j;
    continue;
  }

  i += 1;
}

let patched = out.join('\n');
patched = patched.replaceAll(
  'from_trusted_extension(context, query.tenant_id,',
  'from_trusted_extension(context, query.tenant_id.clone(),',
);

fs.writeFileSync(file, patched);
const missing = (patched.match(/from_trusted_extension\(context/g) ?? []).length;
const extensions = (patched.match(/Extension\(context\): Extension<AgentRequestContext>/g) ?? []).length;
console.log({ trustedCalls: missing, extensionParams: extensions });
