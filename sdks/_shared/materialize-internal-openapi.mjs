import { annotateAgentOpenApiOwnership } from './agent-sdk-ownership.mjs';

export const INTERNAL_OPENAPI_PROBLEM_REF = "          $ref: '#/components/responses/Problem'";
export const INTERNAL_OPENAPI_EXPLICIT_PROBLEM_RESPONSE = `          description: RFC 9457 problem detail response
          content:
            application/problem+json:
              schema:
                $ref: '#/components/schemas/ProblemDetail'`;

export function ensureTrailingNewline(content) {
  return content.endsWith('\n') ? content : `${content}\n`;
}

export function materializeInternalOpenApiAuthority(source, family) {
  return ensureTrailingNewline(annotateAgentOpenApiOwnership(source, family));
}

export function materializeInternalOpenApiSdkgen(authorityYaml, authority = 'sdkwork-agent-internal-api') {
  let output = authorityYaml.replaceAll(
    INTERNAL_OPENAPI_PROBLEM_REF,
    INTERNAL_OPENAPI_EXPLICIT_PROBLEM_RESPONSE
  );
  if (output.includes(INTERNAL_OPENAPI_PROBLEM_REF)) {
    throw new Error(`${authority} sdkgen input still contains response $ref shorthands`);
  }
  return ensureTrailingNewline(output);
}
