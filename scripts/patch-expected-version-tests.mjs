import fs from 'node:fs';

const files = [
  'sdkwork-agent-business/tests/http_axum_contracts.rs',
  'sdkwork-agent-business/src/http.rs'
];

for (const file of files) {
  let src = fs.readFileSync(file, 'utf8');

  src = src.replace(
    /("targetStatus": "[^"]+",)\n(\s+)("requestedAt":)/g,
    (match, statusLine, indent, requestedAtLine) => {
      if (match.includes('expectedVersion')) {
        return match;
      }
      return `${statusLine}\n${indent}"expectedVersion": "1",\n${indent}${requestedAtLine}`;
    }
  );

  src = src.replace(
    /json!\(\{\n(\s+)"requestedAt": "([^"]+)"\n(\s+)\}\)/g,
    (match, indent, requestedAt, closeIndent) => {
      if (match.includes('expectedVersion')) {
        return match;
      }
      return `json!({\n${indent}"expectedVersion": "1",\n${indent}"requestedAt": "${requestedAt}"\n${closeIndent}})`;
    }
  );

  fs.writeFileSync(file, src, 'utf8');
  console.log(`patched ${file}`);
}
