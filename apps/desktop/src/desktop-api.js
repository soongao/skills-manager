import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

const isTauri = Boolean(window.__TAURI_INTERNALS__);

export function detectView() {
  const view = new URLSearchParams(window.location.search).get("view");
  if (view) return view;

  if (!isTauri) return "panel";

  try {
    return getCurrentWindow().label || "panel";
  } catch {
    return "panel";
  }
}

export function createDesktopApi() {
  if (!isTauri) return createPreviewApi();

  return {
    hideCurrentWindow: () => invoke("hide_current_window", {}),
    initConfig: (payload) => invoke("init_config", payload),
    loadDashboard: () => invoke("load_dashboard", {}),
    normalizeError,
    openSettings: () => invoke("open_settings", {}),
    reconcileAll: () => invoke("reconcile", { agentId: null, plan: false }),
    remoteCliStatus: (payload) => invoke("remote_cli_status", payload),
    remoteSync: (payload) => invoke("remote_sync", payload),
    setAgentDir: (payload) => invoke("set_agent_dir", payload),
    setRemoteEnvironment: (payload) => invoke("set_remote_environment", payload),
    setRemoteSource: (payload) => invoke("set_remote_source", payload),
    setSkillEnabled: (payload) => invoke("set_skill_enabled", payload),
    updateOpenCodePath: () =>
      invoke("opencode_ensure_path", { configPathOverride: null }),
  };
}

function createPreviewApi() {
  let dashboard = createPreviewDashboard();

  return {
    hideCurrentWindow: async () => undefined,
    initConfig: async ({ sourceRoot }) => {
      dashboard = {
        ...dashboard,
        sourceRoot,
        config: {
          ...dashboard.config,
          sourceProfiles: [
            {
              kind: "local",
              sourceProfileId: "local-personal",
              sourceRoot,
            },
          ],
        },
      };
      return dashboard;
    },
    loadDashboard: async () => dashboard,
    normalizeError,
    openSettings: async () => {
      window.location.search = "view=settings";
    },
    reconcileAll: async () => dashboard,
    remoteCliStatus: async ({ environmentId }) => ({
      remoteStatus: {
        environmentId,
        remote: "alice@devbox",
        available: true,
        version: { name: "skills-manager", version: "0.1.0" },
      },
    }),
    remoteSync: async ({ direction, plan }) => {
      const remoteLinkPlan =
        direction === "push-local-to-remote"
          ? createPreviewRemoteLinkPlan(dashboard)
          : null;
      const result = {
        [plan ? "plan" : "sync"]: {
        direction,
        commands: [
          {
            source: "/Users/alice/.shared-skills/skills/",
            destination: "alice@devbox:~/.skills-manager/cache/personal/skills/",
            args: ["-az", "--delete"],
          },
        ],
        },
      };
      if (plan) {
        result.remoteLinkPlan = remoteLinkPlan;
      } else if (remoteLinkPlan) {
        result.remoteLink = {
          plan: remoteLinkPlan,
          actions: remoteLinkPlan.links.map((link) => ({
            status: "applied",
            agentId: link.agentId,
            skillId: link.skillId,
            sourcePath: link.sourcePath,
            targetPath: link.targetPath,
            message: null,
          })),
        };
      }
      return result;
    },
    setAgentDir: async ({ environmentId, agentId, skillsDir, managed }) => {
      const environment = previewEnvironment(dashboard, environmentId);
      environment.agents = environment.agents.map((agent) =>
        agent.agentId === agentId ? { ...agent, managed, skillsDir } : agent,
      );
      dashboard.statuses = createStatuses(dashboard.config.environments, dashboard.skills);
      return dashboard;
    },
    setRemoteEnvironment: async (payload) => {
      dashboard.config.environments = dashboard.config.environments.filter(
        (env) => env.environmentId !== payload.environmentId,
      );
      dashboard.config.environments.push({
        environmentId: payload.environmentId,
        kind: "remote",
        host: payload.host,
        user: payload.user,
        syncDirection: payload.direction,
        remoteCacheRoot: payload.remoteCacheRoot,
        autoSync: payload.autoSync,
        deleteExtraneous: payload.deleteExtraneous,
        agents: previewRemoteAgents(payload, dashboard.skills),
      });
      dashboard.statuses = createStatuses(dashboard.config.environments, dashboard.skills);
      return dashboard;
    },
    setRemoteSource: async (payload) => {
      const sourceProfileId = payload.sourceProfileId || "remote-personal";
      dashboard.config.activeSourceProfileId = sourceProfileId;
      dashboard.config.sourceProfiles = dashboard.config.sourceProfiles.filter(
        (profile) => profile.sourceProfileId !== sourceProfileId,
      );
      dashboard.config.sourceProfiles.push({
        kind: "remote",
        sourceProfileId,
        host: payload.host,
        user: payload.user,
        remoteSourceRoot: payload.remoteSourceRoot,
        localCacheRoot: payload.localCacheRoot,
        autoSync: payload.autoSync,
        deleteExtraneous: payload.deleteExtraneous,
      });
      dashboard.sourceRoot = payload.localCacheRoot;
      return dashboard;
    },
    setSkillEnabled: async ({ environmentId, agentId, skillId, enabled }) => {
      const environment = previewEnvironment(dashboard, environmentId);
      environment.agents = environment.agents.map((agent) => {
        if (agent.agentId !== agentId) return agent;
        const ids = new Set(agent.enabledSkillIds);
        if (enabled) ids.add(skillId);
        else ids.delete(skillId);
        return { ...agent, enabledSkillIds: Array.from(ids).sort() };
      });
      dashboard.statuses = createStatuses(dashboard.config.environments, dashboard.skills);
      return dashboard;
    },
    updateOpenCodePath: async () => dashboard,
  };
}

function normalizeError(error) {
  if (typeof error === "string") return error;
  if (error?.message) return error.message;
  return JSON.stringify(error);
}

const AGENT_IDS = ["codex", "claude-code", "opencode"];

function previewEnvironment(dashboard, environmentId) {
  return (
    dashboard.config.environments.find(
      (env) => env.environmentId === (environmentId || "local"),
    ) ?? dashboard.config.environments[0]
  );
}

function previewRemoteAgents(payload, skills) {
  return AGENT_IDS.map((agentId) => ({
    agentId,
    managed: true,
    skillsDir:
      agentId === "codex"
        ? payload.codexSkillsDir
        : agentId === "claude-code"
          ? payload.claudeCodeSkillsDir
          : payload.opencodeSkillsDir,
    enabledSkillIds: skills.map((skill) => skill.skillId),
  })).filter((agent) => agent.skillsDir);
}

function createStatuses(environments, skills) {
  return environments.flatMap((environment) =>
    (environment.agents ?? []).flatMap((agent) =>
      skills.map((skill) => ({
        agentId: agent.agentId,
        environmentId: environment.environmentId,
        skillId: skill.skillId,
        status: agent.enabledSkillIds?.includes(skill.skillId) ? "enabled" : "disabled",
        sourcePath: skill.path,
        targetPath: `${agent.skillsDir}/${skill.skillId}`,
      })),
    ),
  );
}

function createPreviewRemoteLinkPlan(dashboard) {
  const remote = dashboard.config.environments.find((env) => env.kind === "remote");
  if (!remote) return null;
  const sourceRoot = remote.remoteCacheRoot || "~/.skills-manager/cache/personal";
  return {
    host: remote.host,
    user: remote.user,
    environmentId: remote.environmentId,
    sourceRoot,
    links: (remote.agents ?? []).flatMap((agent) =>
      dashboard.skills
        .filter((skill) => agent.enabledSkillIds?.includes(skill.skillId))
        .map((skill) => ({
          agentId: agent.agentId,
          skillId: skill.skillId,
          sourcePath: `${sourceRoot}/skills/${skill.skillId}`,
          targetPath: `${agent.skillsDir}/${skill.skillId}`,
        })),
    ),
  };
}

function createPreviewDashboard() {
  const skills = [
    { skillId: "receiving-code-review", path: "/Users/alice/.shared-skills/skills/receiving-code-review" },
    { skillId: "design-clarifier", path: "/Users/alice/.shared-skills/skills/design-clarifier" },
    { skillId: "skill-installer", path: "/Users/alice/.shared-skills/skills/skill-installer" },
    { skillId: "writing-great-skills", path: "/Users/alice/.shared-skills/skills/writing-great-skills" },
  ];

  const agents = [
    {
      agentId: "codex",
      managed: true,
      skillsDir: "/Users/alice/.codex/skills",
      enabledSkillIds: skills.map((skill) => skill.skillId),
    },
    {
      agentId: "claude-code",
      managed: true,
      skillsDir: "/Users/alice/.claude/skills",
      enabledSkillIds: skills.map((skill) => skill.skillId),
    },
    {
      agentId: "opencode",
      managed: true,
      skillsDir: "/Users/alice/.config/opencode/skills",
      enabledSkillIds: skills.map((skill) => skill.skillId),
    },
  ];

  return {
    configHome: "/Users/alice/.skills-manager",
    sourceRoot: "/Users/alice/.shared-skills",
    config: {
      schemaVersion: 1,
      activeSourceProfileId: "local-personal",
      sourceProfiles: [
        {
          kind: "local",
          sourceProfileId: "local-personal",
          sourceRoot: "/Users/alice/.shared-skills",
        },
      ],
      environments: [
        {
          environmentId: "local",
          kind: "local",
          agents,
        },
        {
          environmentId: "devbox",
          kind: "remote",
          host: "devbox",
          user: "alice",
          syncDirection: "push-local-to-remote",
          remoteCacheRoot: "~/.skills-manager/cache/personal",
          autoSync: false,
          deleteExtraneous: true,
          agents: previewRemoteAgents(
            {
              codexSkillsDir: "~/.codex/skills",
              claudeCodeSkillsDir: "~/.claude/skills",
              opencodeSkillsDir: "~/.config/opencode/skills",
            },
            skills,
          ),
        },
      ],
    },
    detection: {
      ssh: { status: "available" },
      rsync: { status: "available" },
      agents: agents.map((agent) => ({
        agentId: agent.agentId,
        command: { status: "available" },
        recommendedSkillsDir: agent.skillsDir,
      })),
    },
    skills,
    statuses: createStatuses(
      [
        {
          environmentId: "local",
          agents,
        },
        {
          environmentId: "devbox",
          agents: previewRemoteAgents(
            {
              codexSkillsDir: "~/.codex/skills",
              claudeCodeSkillsDir: "~/.claude/skills",
              opencodeSkillsDir: "~/.config/opencode/skills",
            },
            skills,
          ),
        },
      ],
      skills,
    ),
    hooks: [
      { agentId: "codex", status: "separate", reason: "Codex reads the configured skills directory directly." },
      { agentId: "claude-code", status: "planned", reason: "Session-start hook applies links before the agent loads skills." },
      { agentId: "opencode", status: "separate", reason: "OpenCode path is updated in its own configuration file." },
    ],
  };
}
