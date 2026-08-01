const REQUEST_METHODS = Object.freeze({
  'item/commandExecution/requestApproval': Object.freeze({
    category: 'approval',
    kind: 'command_execution',
  }),
  'item/fileChange/requestApproval': Object.freeze({
    category: 'approval',
    kind: 'file_change',
  }),
  'item/permissions/requestApproval': Object.freeze({
    category: 'approval',
    kind: 'permission_profile',
  }),
  'item/tool/requestUserInput': Object.freeze({
    category: 'user_input',
    kind: 'question_set',
  }),
  'item/tool/requestOptionPicker': Object.freeze({
    category: 'user_input',
    kind: 'option_picker',
  }),
  'item/tool/requestSetupCodexContextPicker': Object.freeze({
    category: 'user_input',
    kind: 'context_source_picker',
  }),
  'mcpServer/elicitation/request': Object.freeze({
    category: 'elicitation',
    kind: 'mcp_elicitation',
  }),
});

const DYNAMIC_TOOL_METHOD = 'item/tool/call';
const ONBOARDING_INPUT_TOOL = 'request_onboarding_input';
const OPTION_PICKER_TOOL = 'request_option_picker';
const SETUP_CONTEXT_PICKER_TOOL = 'setup_codex_context_picker';
const SETUP_STEP_TOOL = 'setup_codex_step';
const SETUP_STEPS = Object.freeze(['role', 'task', 'context', 'complete']);
const DYNAMIC_INTERACTION_TOOLS = new Set([
  ONBOARDING_INPUT_TOOL,
  OPTION_PICKER_TOOL,
  SETUP_CONTEXT_PICKER_TOOL,
  SETUP_STEP_TOOL,
]);

const COMMAND_ACTIONS = Object.freeze([
  'accept',
  'accept_for_session',
  'accept_with_exec_policy_amendment',
  'apply_network_policy_amendment',
  'decline',
  'cancel',
]);
const FILE_CHANGE_ACTIONS = Object.freeze([
  'accept',
  'accept_for_session',
  'decline',
  'cancel',
]);
const QUESTION_ACTIONS = Object.freeze(['submit', 'cancel']);
const OPTION_PICKER_ACTIONS = Object.freeze(['submit', 'skip', 'dismiss']);
const CONTEXT_PICKER_ACTIONS = Object.freeze(['continue', 'skip', 'dismiss']);
const SETUP_ROLE_TASK_ACTIONS = Object.freeze(['submit', 'skip', 'dismiss']);
const ELICITATION_ACTIONS = Object.freeze(['accept', 'decline', 'cancel']);
const PERMISSION_ACTIONS = Object.freeze(['grant', 'decline', 'cancel']);

export class CodexInteractionProtocolError extends Error {
  constructor(code, message) {
    super(`${code}: ${message}`);
    this.name = 'CodexInteractionProtocolError';
    this.code = code;
  }
}

export function normalizeCodexInteractionRequest(event, context = {}) {
  const source = requiredRecord(event, 'request event');
  const method = requiredString(source.method, 'method');
  const params = requiredRecord(source.params ?? {}, 'params');
  const profile = resolveRequestProfile(method, params);
  const requestId = providerRequestId(source.requestId);
  const approvalId = optionalString(params.approvalId, 'approvalId');
  const providerSessionId = optionalString(
    source.providerSessionId ?? params.providerSessionId,
    'providerSessionId',
  );
  const providerTurnId = optionalString(source.turnId ?? params.turnId, 'turnId');
  const providerItemId = optionalString(params.itemId ?? params.callId, 'itemId');
  const interactionId = approvalId ?? String(requestId);
  const sessionId = optionalString(context.sessionId, 'sessionId');
  const modelRequestId = optionalString(context.modelRequestId, 'modelRequestId');

  return {
    schemaVersion: 1,
    interactionId,
    sessionId,
    category: profile.category,
    kind: profile.kind,
    prompt: interactionPrompt(profile.kind, profile.payload),
    allowedActions: allowedActions(profile.kind, profile.payload),
    request: normalizeRequestPayload(profile.kind, profile.payload),
    correlation: {
      modelRequestId,
      providerId: 'codex',
      providerInteractionId: approvalId,
      providerItemId,
      providerRequestId: requestId,
      providerRequestIdType: typeof requestId,
      providerSessionId,
      providerToolCallId: profile.toolCallId,
      providerToolName: profile.toolName,
      providerToolNamespace: profile.toolNamespace,
      providerTurnId,
      protocolMethod: method,
    },
    receivedAt: optionalString(source.receivedAt, 'receivedAt'),
  };
}

export function projectCodexInteractionServerRequest(event, context = {}) {
  const source = requiredRecord(event, 'request event');
  const method = requiredString(source.method, 'method');
  if (method !== DYNAMIC_TOOL_METHOD) {
    return {
      disposition: 'interaction',
      interaction: normalizeCodexInteractionRequest(source, context),
    };
  }

  const params = requiredRecord(source.params ?? {}, 'params');
  const tool = requiredString(params.tool, 'tool');
  if (!DYNAMIC_INTERACTION_TOOLS.has(tool)) {
    throw protocolError(
      'codex_interaction_unsupported_method',
      `dynamic tool ${tool} requires a typed Kernel host-tool port`,
    );
  }

  try {
    if (tool === SETUP_STEP_TOOL) {
      const setup = normalizeSetupStepArguments(params.arguments);
      if (setup.step === 'complete') {
        return {
          disposition: 'automatic_response',
          result: dynamicToolSuccessResponse({ completed: true }),
        };
      }
    }
    return {
      disposition: 'interaction',
      interaction: normalizeCodexInteractionRequest(source, context),
    };
  } catch (error) {
    if (
      error?.code === 'codex_interaction_invalid_request'
      && [OPTION_PICKER_TOOL, ONBOARDING_INPUT_TOOL, SETUP_STEP_TOOL].includes(tool)
    ) {
      return {
        disposition: 'automatic_response',
        result: dynamicToolFailureResponse(`${tool} received invalid arguments.`),
      };
    }
    throw error;
  }
}

export function buildCodexInteractionResponse(interaction, resolution) {
  const normalizedInteraction = requiredRecord(interaction, 'interaction');
  const normalizedResolution = requiredRecord(
    resolution,
    'resolution',
    'codex_interaction_invalid_resolution',
  );
  const action = requiredString(
    normalizedResolution.action,
    'resolution.action',
    'codex_interaction_invalid_resolution',
  );

  let response;
  switch (normalizedInteraction.kind) {
    case 'command_execution':
      response = commandResponse(action, normalizedResolution);
      break;
    case 'file_change':
      response = fileChangeResponse(action);
      break;
    case 'question_set':
    case 'onboarding_question_set':
      response = questionResponse(normalizedInteraction, action, normalizedResolution);
      break;
    case 'option_picker':
      response = optionPickerResponse(action, normalizedResolution);
      break;
    case 'context_source_picker':
      response = contextSourcePickerResponse(action, normalizedResolution);
      break;
    case 'setup_step':
      response = setupStepResponse(normalizedInteraction, action, normalizedResolution);
      break;
    case 'mcp_elicitation':
      response = elicitationResponse(action, normalizedResolution);
      break;
    case 'permission_profile':
      response = permissionResponse(action, normalizedResolution);
      break;
    default:
      throw protocolError(
        'codex_interaction_unsupported_kind',
        `unsupported canonical interaction kind ${String(normalizedInteraction.kind)}`,
      );
  }
  return normalizedInteraction.correlation?.protocolMethod === DYNAMIC_TOOL_METHOD
    ? dynamicToolSuccessResponse(response)
    : response;
}

function resolveRequestProfile(method, params) {
  const directProfile = REQUEST_METHODS[method];
  if (directProfile) {
    return {
      ...directProfile,
      payload: params,
      toolCallId: null,
      toolName: null,
      toolNamespace: null,
    };
  }
  if (method !== DYNAMIC_TOOL_METHOD) {
    throw protocolError(
      'codex_interaction_unsupported_method',
      `unsupported user-mediated request method ${method}`,
    );
  }

  const toolName = requiredString(params.tool, 'tool');
  const toolCallId = requiredString(params.callId, 'callId');
  const toolNamespace = optionalNullableStringValue(params.namespace, 'namespace');
  const common = { itemId: toolCallId };
  if (toolName === ONBOARDING_INPUT_TOOL) {
    return {
      category: 'user_input',
      kind: 'onboarding_question_set',
      payload: { ...common, ...normalizeOnboardingArguments(params.arguments) },
      toolCallId,
      toolName,
      toolNamespace,
    };
  }
  if (toolName === OPTION_PICKER_TOOL) {
    return {
      category: 'user_input',
      kind: 'option_picker',
      payload: { ...common, ...normalizeOptionPickerArguments(params.arguments) },
      toolCallId,
      toolName,
      toolNamespace,
    };
  }
  if (toolName === SETUP_CONTEXT_PICKER_TOOL) {
    requiredJson(params.arguments, 'arguments');
    return {
      category: 'user_input',
      kind: 'context_source_picker',
      payload: common,
      toolCallId,
      toolName,
      toolNamespace,
    };
  }
  if (toolName === SETUP_STEP_TOOL) {
    const setup = normalizeSetupStepArguments(params.arguments);
    if (setup.step === 'complete') {
      throw protocolError(
        'codex_interaction_unsupported_method',
        'completed setup is a host-mediated response, not a user Interaction',
      );
    }
    return {
      category: 'setup',
      kind: 'setup_step',
      payload: { ...common, ...setup },
      toolCallId,
      toolName,
      toolNamespace,
    };
  }
  throw protocolError(
    'codex_interaction_unsupported_method',
    `dynamic tool ${toolName} requires a typed Kernel host-tool port`,
  );
}

function normalizeRequestPayload(kind, params) {
  const common = {
    environmentId: optionalString(params.environmentId, 'environmentId'),
    itemId: optionalString(params.itemId, 'itemId'),
    reason: optionalString(params.reason, 'reason'),
    startedAtMs: optionalSafeInteger(params.startedAtMs, 'startedAtMs'),
  };
  switch (kind) {
    case 'command_execution':
      return {
        ...common,
        command: optionalString(params.command, 'command'),
        cwd: optionalString(params.cwd, 'cwd'),
        commandActions: optionalArray(params.commandActions, 'commandActions'),
        networkContext: optionalJson(params.networkApprovalContext),
        proposedExecPolicyAmendment: optionalJson(params.proposedExecpolicyAmendment),
        proposedNetworkPolicyAmendments: optionalArray(
          params.proposedNetworkPolicyAmendments,
          'proposedNetworkPolicyAmendments',
        ),
      };
    case 'file_change':
      return {
        ...common,
        grantRoot: optionalString(params.grantRoot, 'grantRoot'),
      };
    case 'permission_profile':
      return {
        ...common,
        cwd: requiredString(params.cwd, 'cwd'),
        requestedPermissions: cloneJson(requiredRecord(params.permissions, 'permissions')),
      };
    case 'question_set':
      return {
        itemId: common.itemId,
        autoResolutionMs: nullableNonNegativeInteger(
          params.autoResolutionMs,
          'autoResolutionMs',
        ),
        isBlocking: params.isBlocking == null
          ? true
          : requiredBoolean(params.isBlocking, 'isBlocking'),
        questions: normalizeQuestions(params.questions),
      };
    case 'onboarding_question_set':
      return {
        itemId: common.itemId,
        autoResolutionMs: null,
        isBlocking: true,
        presentation: 'onboarding',
        questions: params.questions,
      };
    case 'option_picker':
      return {
        itemId: common.itemId,
        question: requiredStringValue(params.question, 'question'),
        options: normalizePickerOptions(params.options, 'options'),
        allowMultiple: params.allowMultiple == null
          ? false
          : requiredBoolean(params.allowMultiple, 'allowMultiple'),
        submitLabel: optionalNullableStringValue(params.submitLabel, 'submitLabel'),
        skipLabel: optionalNullableStringValue(params.skipLabel, 'skipLabel'),
      };
    case 'context_source_picker':
      return { itemId: common.itemId };
    case 'setup_step':
      return {
        itemId: common.itemId,
        step: requiredSetupStep(params.step, 'step'),
      };
    case 'mcp_elicitation':
      return normalizeElicitation(params);
    default:
      throw protocolError(
        'codex_interaction_unsupported_kind',
        `unsupported canonical interaction kind ${kind}`,
      );
  }
}

function normalizeOptionPickerArguments(value) {
  const args = requiredRecord(value, 'arguments');
  return {
    question: requiredStringValue(args.question, 'arguments.question'),
    options: normalizePickerOptions(args.options, 'arguments.options'),
    allowMultiple: args.allowMultiple == null
      ? false
      : requiredBoolean(args.allowMultiple, 'arguments.allowMultiple'),
    submitLabel: optionalNullableStringValue(args.submitLabel, 'arguments.submitLabel'),
    skipLabel: optionalNullableStringValue(args.skipLabel, 'arguments.skipLabel'),
  };
}

function normalizeOnboardingArguments(value) {
  const args = requiredRecord(value, 'arguments');
  assertOnlyKeys(args, ['questions'], 'arguments');
  const questions = requiredArray(args.questions, 'arguments.questions');
  if (questions.length < 1 || questions.length > 3) {
    throw protocolError(
      'codex_interaction_invalid_request',
      'arguments.questions must contain between one and three questions',
    );
  }
  const ids = new Set();
  return {
    questions: questions.map((entry, index) => {
      const field = `arguments.questions[${index}]`;
      const question = requiredRecord(entry, field);
      assertOnlyKeys(question, ['id', 'header', 'question', 'options'], field);
      const id = requiredStringValue(question.id, `${field}.id`);
      if (ids.has(id)) {
        throw protocolError('codex_interaction_invalid_request', `duplicate question id ${id}`);
      }
      ids.add(id);
      const prompt = requiredStringValue(question.question, `${field}.question`);
      const options = normalizePickerOptions(question.options, `${field}.options`);
      if (options.length < 2) {
        throw protocolError(
          'codex_interaction_invalid_request',
          `${field}.options must contain at least two options`,
        );
      }
      return {
        id,
        header: optionalNullableStringValue(question.header, `${field}.header`) ?? prompt,
        prompt,
        allowOther: true,
        secret: false,
        options: options.map((option) => ({
          label: option.label,
          description: option.description ?? '',
        })),
      };
    }),
  };
}

function normalizePickerOptions(value, fieldName) {
  return requiredArray(value, fieldName).map((entry, index) => {
    const option = requiredRecord(entry, `${fieldName}[${index}]`);
    return {
      label: requiredStringValue(option.label, `${fieldName}[${index}].label`),
      description: optionalNullableStringValue(
        option.description,
        `${fieldName}[${index}].description`,
      ),
    };
  });
}

function normalizeSetupStepArguments(value) {
  const args = requiredRecord(value, 'arguments');
  assertOnlyKeys(args, ['step'], 'arguments');
  return { step: requiredSetupStep(args.step, 'arguments.step') };
}

function normalizeQuestions(value) {
  if (!Array.isArray(value) || value.length === 0) {
    throw protocolError('codex_interaction_invalid_request', 'questions must be non-empty');
  }
  const ids = new Set();
  return value.map((entry, index) => {
    const question = requiredRecord(entry, `questions[${index}]`);
    const id = requiredString(question.id, `questions[${index}].id`);
    if (ids.has(id)) {
      throw protocolError('codex_interaction_invalid_request', `duplicate question id ${id}`);
    }
    ids.add(id);
    return {
      id,
      header: requiredString(question.header, `questions[${index}].header`),
      prompt: requiredString(question.question, `questions[${index}].question`),
      allowOther: requiredBoolean(question.isOther, `questions[${index}].isOther`),
      secret: requiredBoolean(question.isSecret, `questions[${index}].isSecret`),
      options: question.options == null
        ? null
        : requiredArray(question.options, `questions[${index}].options`).map(
            (option, optionIndex) => {
              const item = requiredRecord(
                option,
                `questions[${index}].options[${optionIndex}]`,
              );
              return {
                label: requiredString(
                  item.label,
                  `questions[${index}].options[${optionIndex}].label`,
                ),
                description: requiredStringValue(
                  item.description,
                  `questions[${index}].options[${optionIndex}].description`,
                ),
              };
            },
          ),
    };
  });
}

function normalizeElicitation(params) {
  const mode = requiredString(params.mode, 'mode');
  if (!['form', 'openai/form', 'url'].includes(mode)) {
    throw protocolError('codex_interaction_invalid_request', `unsupported elicitation mode ${mode}`);
  }
  const request = {
    serverName: requiredString(params.serverName, 'serverName'),
    mode,
    message: requiredString(params.message, 'message'),
    metadata: optionalJson(params._meta),
  };
  if (mode === 'url') {
    return {
      ...request,
      elicitationId: requiredString(params.elicitationId, 'elicitationId'),
      url: requiredString(params.url, 'url'),
    };
  }
  return {
    ...request,
    requestedSchema: mode === 'form'
      ? cloneJson(requiredRecord(params.requestedSchema, 'requestedSchema'))
      : requiredJson(params.requestedSchema, 'requestedSchema'),
  };
}

function commandResponse(action, resolution) {
  assertAllowedAction('command_execution', action, COMMAND_ACTIONS);
  if (action === 'accept_for_session') return { decision: 'acceptForSession' };
  if (action === 'accept_with_exec_policy_amendment') {
    return {
      decision: {
        acceptWithExecpolicyAmendment: {
          execpolicy_amendment: cloneJson(
            requiredRecord(
              resolution.execPolicyAmendment,
              'resolution.execPolicyAmendment',
              'codex_interaction_invalid_resolution',
            ),
            'codex_interaction_invalid_resolution',
          ),
        },
      },
    };
  }
  if (action === 'apply_network_policy_amendment') {
    return {
      decision: {
        applyNetworkPolicyAmendment: {
          network_policy_amendment: cloneJson(
            requiredRecord(
              resolution.networkPolicyAmendment,
              'resolution.networkPolicyAmendment',
              'codex_interaction_invalid_resolution',
            ),
            'codex_interaction_invalid_resolution',
          ),
        },
      },
    };
  }
  return { decision: action };
}

function fileChangeResponse(action) {
  assertAllowedAction('file_change', action, FILE_CHANGE_ACTIONS);
  return { decision: action === 'accept_for_session' ? 'acceptForSession' : action };
}

function questionResponse(interaction, action, resolution) {
  assertAllowedAction(interaction.kind, action, QUESTION_ACTIONS);
  if (action === 'cancel') return { answers: {} };
  const answers = requiredRecord(
    resolution.answers,
    'resolution.answers',
    'codex_interaction_invalid_resolution',
  );
  const knownIds = new Set((interaction.request?.questions ?? []).map((question) => question.id));
  const normalized = {};
  for (const [questionId, values] of Object.entries(answers)) {
    if (!knownIds.has(questionId)) {
      throw protocolError(
        'codex_interaction_invalid_resolution',
        `answer references unknown question id ${questionId}`,
      );
    }
    normalized[questionId] = {
      answers: requiredArray(
        values,
        `resolution.answers.${questionId}`,
        'codex_interaction_invalid_resolution',
      ).map(
        (value, index) => requiredStringValue(
          value,
          `resolution.answers.${questionId}[${index}]`,
          'codex_interaction_invalid_resolution',
        ),
      ),
    };
  }
  return { answers: normalized };
}

function optionPickerResponse(action, resolution) {
  assertAllowedAction('option_picker', action, OPTION_PICKER_ACTIONS);
  return {
    action,
    selectedOptions: stringArray(
      resolution.selectedOptions,
      'resolution.selectedOptions',
      'codex_interaction_invalid_resolution',
    ),
    freeformAnswer: optionalNullableStringValue(
      resolution.freeformAnswer,
      'resolution.freeformAnswer',
      'codex_interaction_invalid_resolution',
    ),
  };
}

function contextSourcePickerResponse(action, resolution) {
  assertAllowedAction('context_source_picker', action, CONTEXT_PICKER_ACTIONS);
  return {
    action,
    selectedSources: stringArray(
      resolution.selectedSources,
      'resolution.selectedSources',
      'codex_interaction_invalid_resolution',
    ),
  };
}

function setupStepResponse(interaction, action, resolution) {
  const step = requiredSetupStep(
    interaction.request?.step,
    'interaction.request.step',
    'codex_interaction_invalid_resolution',
  );
  if (step === 'role') {
    assertAllowedAction('setup_step.role', action, SETUP_ROLE_TASK_ACTIONS);
    return {
      action,
      selectedRoles: stringArray(
        resolution.selectedRoles,
        'resolution.selectedRoles',
        'codex_interaction_invalid_resolution',
      ),
    };
  }
  if (step === 'task') {
    assertAllowedAction('setup_step.task', action, SETUP_ROLE_TASK_ACTIONS);
    return {
      action,
      answers: nestedAnswerMap(
        resolution.answers,
        'resolution.answers',
        'codex_interaction_invalid_resolution',
      ),
    };
  }
  if (step === 'context') {
    assertAllowedAction('setup_step.context', action, CONTEXT_PICKER_ACTIONS);
    return {
      action,
      selectedSources: stringArray(
        resolution.selectedSources,
        'resolution.selectedSources',
        'codex_interaction_invalid_resolution',
      ),
    };
  }
  throw protocolError(
    'codex_interaction_invalid_resolution',
    `setup step ${step} is not user-mediated`,
  );
}

function elicitationResponse(action, resolution) {
  assertAllowedAction('mcp_elicitation', action, ELICITATION_ACTIONS);
  return {
    action,
    content: action === 'accept'
      ? optionalJson(resolution.content, 'codex_interaction_invalid_resolution')
      : null,
    _meta: optionalJson(resolution.metadata, 'codex_interaction_invalid_resolution'),
  };
}

function permissionResponse(action, resolution) {
  assertAllowedAction('permission_profile', action, PERMISSION_ACTIONS);
  if (action !== 'grant') return { permissions: {}, scope: 'turn' };
  const scope = requiredString(
    resolution.scope,
    'resolution.scope',
    'codex_interaction_invalid_resolution',
  );
  if (!['turn', 'session'].includes(scope)) {
    throw protocolError(
      'codex_interaction_invalid_resolution',
      `permission scope must be turn or session, received ${scope}`,
    );
  }
  const response = {
    permissions: cloneJson(
      requiredRecord(
        resolution.permissions,
        'resolution.permissions',
        'codex_interaction_invalid_resolution',
      ),
      'codex_interaction_invalid_resolution',
    ),
    scope,
  };
  if (resolution.strictAutoReview != null) {
    response.strictAutoReview = requiredBoolean(
      resolution.strictAutoReview,
      'resolution.strictAutoReview',
      'codex_interaction_invalid_resolution',
    );
  }
  return response;
}

function interactionPrompt(kind, params) {
  const reason = optionalString(params.reason, 'reason');
  if (reason) return reason;
  if (kind === 'command_execution') {
    const command = optionalString(params.command, 'command');
    return command ? `Run command: ${command}` : 'Allow command execution';
  }
  if (kind === 'file_change') return 'Allow file changes';
  if (kind === 'permission_profile') return 'Grant requested permissions';
  if (kind === 'question_set' || kind === 'onboarding_question_set') {
    const first = Array.isArray(params.questions) ? params.questions[0] : null;
    return optionalString(
      first?.question ?? first?.prompt,
      'questions[0].question',
    ) ?? 'Answer questions';
  }
  if (kind === 'option_picker') {
    return optionalString(params.question, 'question') ?? 'Choose an option';
  }
  if (kind === 'context_source_picker') return 'Choose context sources';
  if (kind === 'setup_step') return `Complete setup step: ${params.step}`;
  return optionalString(params.message, 'message') ?? 'Respond to tool request';
}

function allowedActions(kind, params) {
  const actions = {
    command_execution: COMMAND_ACTIONS,
    context_source_picker: CONTEXT_PICKER_ACTIONS,
    file_change: FILE_CHANGE_ACTIONS,
    mcp_elicitation: ELICITATION_ACTIONS,
    onboarding_question_set: QUESTION_ACTIONS,
    option_picker: OPTION_PICKER_ACTIONS,
    permission_profile: PERMISSION_ACTIONS,
    question_set: QUESTION_ACTIONS,
    setup_step: params.step === 'context' ? CONTEXT_PICKER_ACTIONS : SETUP_ROLE_TASK_ACTIONS,
  }[kind];
  if (!actions) {
    throw protocolError(
      'codex_interaction_unsupported_kind',
      `unsupported canonical interaction kind ${kind}`,
    );
  }
  return [...actions];
}

function assertAllowedAction(kind, action, allowed) {
  if (!allowed.includes(action)) {
    throw protocolError(
      'codex_interaction_invalid_resolution',
      `${kind} does not allow action ${action}`,
    );
  }
}

function providerRequestId(value) {
  if (typeof value === 'string' && value.trim()) return value;
  if (typeof value === 'number' && Number.isSafeInteger(value)) return value;
  throw protocolError(
    'codex_interaction_invalid_request',
    'requestId must be a non-empty string or safe integer',
  );
}

function requiredRecord(value, fieldName, errorCode = 'codex_interaction_invalid_request') {
  if (!isRecord(value)) {
    throw protocolError(errorCode, `${fieldName} must be an object`);
  }
  return value;
}

function requiredArray(value, fieldName, errorCode = 'codex_interaction_invalid_request') {
  if (!Array.isArray(value)) {
    throw protocolError(errorCode, `${fieldName} must be an array`);
  }
  return value;
}

function optionalArray(value, fieldName) {
  if (value == null) return null;
  if (!Array.isArray(value)) {
    throw protocolError('codex_interaction_invalid_request', `${fieldName} must be an array`);
  }
  return cloneJson(value);
}

function requiredString(value, fieldName, errorCode = 'codex_interaction_invalid_request') {
  const normalized = optionalString(value, fieldName, errorCode);
  if (!normalized) {
    throw protocolError(errorCode, `${fieldName} is required`);
  }
  return normalized;
}

function requiredStringValue(value, fieldName, errorCode = 'codex_interaction_invalid_request') {
  if (typeof value !== 'string') {
    throw protocolError(errorCode, `${fieldName} must be a string`);
  }
  return value;
}

function optionalNullableStringValue(
  value,
  fieldName,
  errorCode = 'codex_interaction_invalid_request',
) {
  if (value == null) return null;
  return requiredStringValue(value, fieldName, errorCode);
}

function optionalString(value, fieldName, errorCode = 'codex_interaction_invalid_request') {
  if (value == null) return null;
  if (typeof value !== 'string') {
    throw protocolError(errorCode, `${fieldName} must be a string`);
  }
  return value.trim() || null;
}

function requiredBoolean(value, fieldName, errorCode = 'codex_interaction_invalid_request') {
  if (typeof value !== 'boolean') {
    throw protocolError(errorCode, `${fieldName} must be a boolean`);
  }
  return value;
}

function requiredSetupStep(value, fieldName, errorCode = 'codex_interaction_invalid_request') {
  const step = requiredStringValue(value, fieldName, errorCode);
  if (!SETUP_STEPS.includes(step)) {
    throw protocolError(errorCode, `${fieldName} must be role, task, context, or complete`);
  }
  return step;
}

function stringArray(value, fieldName, errorCode) {
  return requiredArray(value, fieldName, errorCode).map((entry, index) =>
    requiredStringValue(entry, `${fieldName}[${index}]`, errorCode));
}

function nestedAnswerMap(value, fieldName, errorCode) {
  const answers = requiredRecord(value, fieldName, errorCode);
  return Object.fromEntries(Object.entries(answers).map(([questionId, entry]) => {
    const answer = requiredRecord(entry, `${fieldName}.${questionId}`, errorCode);
    assertOnlyKeys(answer, ['answers'], `${fieldName}.${questionId}`, errorCode);
    return [questionId, {
      answers: stringArray(answer.answers, `${fieldName}.${questionId}.answers`, errorCode),
    }];
  }));
}

function optionalSafeInteger(value, fieldName) {
  if (value == null) return null;
  if (!Number.isSafeInteger(value)) {
    throw protocolError('codex_interaction_invalid_request', `${fieldName} must be a safe integer`);
  }
  return value;
}

function nullableNonNegativeInteger(value, fieldName) {
  if (value == null) return null;
  if (!Number.isSafeInteger(value) || value < 0) {
    throw protocolError(
      'codex_interaction_invalid_request',
      `${fieldName} must be a non-negative safe integer or null`,
    );
  }
  return value;
}

function optionalJson(value, errorCode = 'codex_interaction_invalid_request') {
  return value == null ? null : cloneJson(value, errorCode);
}

function requiredJson(value, fieldName, errorCode = 'codex_interaction_invalid_request') {
  if (value === undefined) {
    throw protocolError(errorCode, `${fieldName} is required`);
  }
  return cloneJson(value, errorCode);
}

function cloneJson(value, errorCode = 'codex_interaction_invalid_request') {
  if (Array.isArray(value)) return value.map((entry) => cloneJson(entry, errorCode));
  if (isRecord(value)) {
    return Object.fromEntries(
      Object.entries(value).map(([key, entry]) => [key, cloneJson(entry, errorCode)]),
    );
  }
  if (
    value === null
    || typeof value === 'string'
    || typeof value === 'boolean'
    || (typeof value === 'number' && Number.isFinite(value))
  ) {
    return value;
  }
  throw protocolError(errorCode, 'value must be JSON-compatible');
}

function assertOnlyKeys(
  value,
  allowedKeys,
  fieldName,
  errorCode = 'codex_interaction_invalid_request',
) {
  const allowed = new Set(allowedKeys);
  const unknown = Object.keys(value).filter((key) => !allowed.has(key));
  if (unknown.length > 0) {
    throw protocolError(errorCode, `${fieldName} contains unknown field ${unknown[0]}`);
  }
}

function dynamicToolSuccessResponse(payload) {
  return {
    contentItems: [{ type: 'inputText', text: JSON.stringify(payload) }],
    success: true,
  };
}

function dynamicToolFailureResponse(message) {
  return {
    contentItems: [{ type: 'inputText', text: message }],
    success: false,
  };
}

function isRecord(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function protocolError(code, message) {
  return new CodexInteractionProtocolError(code, message);
}
