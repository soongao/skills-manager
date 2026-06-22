import { AGENTS } from "../constants.js";
import { escapeAttr, escapeHtml, shortPath } from "../format.js";

const SETTINGS_PAGES = [
  ["general", "General", "Status"],
  ["skills", "Skills", "Library"],
  ["agents", "Agents", "Targets"],
  ["remote", "Remote", "Sources"],
  ["sync", "Sync", "Transport"],
  ["advanced", "Advanced", "Hooks"],
];

export function renderSettings(model, state) {
  const page = state.settingsPage || "general";

  return `
    <section class="settings-shell mac-shell">
      <aside class="mac-sidebar">
        <div class="sidebar-title">
          <span class="app-glyph">SM</span>
          <div>
            <strong>Skills Manager</strong>
            <small>${model.configured ? "Personal library connected" : "Setup required"}</small>
          </div>
        </div>
        <nav class="settings-nav" aria-label="Settings">
          ${SETTINGS_PAGES.map(([id, label, hint]) => renderNavItem(id, label, hint, page)).join("")}
        </nav>
      </aside>

      <main class="mac-content">
        <header class="mac-toolbar">
          <div class="toolbar-copy">
            <h1>${pageTitle(page)}</h1>
            <p>${pageSubtitle(page)}</p>
          </div>
          <div class="toolbar-actions">
            <span class="toolbar-state ${state.busy ? "working" : ""}">${state.busy ? "Working" : "Idle"}</span>
            <button class="mac-button" data-action="refresh" ${state.busy ? "disabled" : ""}>Refresh</button>
            <button class="mac-button primary" data-action="reconcile-all" ${state.busy || !model.configured ? "disabled" : ""}>Apply</button>
          </div>
        </header>

        <section class="page-body">
          ${state.error ? `<div class="mac-alert">${escapeHtml(state.error)}</div>` : ""}
          ${model.configured ? renderPage(page, model, state) : renderSetupPage(state)}
        </section>
      </main>
    </section>
  `;
}

function renderNavItem(id, label, hint, activePage) {
  return `
    <button class="nav-row ${activePage === id ? "active" : ""}" data-action="select-settings-page" data-page="${id}" type="button">
      <span class="nav-mark">${navMark(id)}</span>
      <span>
        <strong>${label}</strong>
        <small>${hint}</small>
      </span>
    </button>
  `;
}

function renderPage(page, model, state) {
  switch (page) {
    case "skills":
      return renderSkillsPage(model, state);
    case "agents":
      return renderAgentsPage(model, state);
    case "remote":
      return renderRemotePage(model, state);
    case "sync":
      return renderSyncPage(model, state);
    case "advanced":
      return renderAdvancedPage(model, state);
    case "general":
    default:
      return renderGeneralPage(model);
  }
}

function renderGeneralPage(model) {
  return `
    <div class="settings-page">
      <section class="overview-hero">
        <div class="status-orb ${model.conflictCount ? "bad" : "good"}">
          <span>${model.conflictCount ? "!" : "OK"}</span>
        </div>
        <div class="overview-copy">
          <p class="section-label">Shared source</p>
          <h2>${escapeHtml(model.healthLabel)}</h2>
          <p>${model.skills.length} skills are available from ${escapeHtml(shortPath(model.sourceRoot))}.</p>
        </div>
        <span class="status-pill ${model.conflictCount ? "bad" : "good"}">${model.conflictCount ? `${model.conflictCount} conflicts` : "Ready"}</span>
      </section>

      <section class="metric-grid">
        ${renderMetricTile("Skills", model.skills.length, "Detected in source")}
        ${renderMetricTile("Links", model.enabledCount, "Enabled across agents")}
        ${renderMetricTile("Conflicts", model.conflictCount, "Skipped until resolved", model.conflictCount ? "bad" : "")}
      </section>

      ${renderPreferenceBlock("Locations", "Folder choices used by local agents.", `
        ${renderInfoRow("Shared source", model.sourceRoot)}
        ${renderInfoRow("Config home", model.dashboard?.configHome)}
        ${renderInfoRow("Active profile", model.dashboard?.config?.activeSourceProfileId)}
      `)}

      <section class="agent-summary-grid">
        ${AGENTS.map(([agentId, label]) => renderAgentSummary(agentId, label, model)).join("")}
      </section>
    </div>
  `;
}

function renderSkillsPage(model, state) {
  const selectedSkill = model.skills.find((skill) => skill.skillId === state.selectedSkillId);
  if (selectedSkill) return renderSkillDetailPage(selectedSkill, model);

  return `
    <div class="settings-page">
      ${renderSectionIntro("Skill library", "Open a skill to choose which local and remote agents can use it.")}
      <section class="pref-group skill-list">
        ${model.skills.map((skill) => renderSkillPreference(skill, model)).join("") || renderEmptyState("No skills found", "Choose a shared source folder that contains skill directories.")}
      </section>
    </div>
  `;
}

function renderSkillPreference(skill, model) {
  const totalAgents = allConfiguredAgents(model).length;
  const enabledAgents = allConfiguredAgents(model).filter(({ agent }) =>
    agent.enabledSkillIds?.includes(skill.skillId),
  );

  return `
    <button class="pref-row skill-pref skill-open-row" data-action="select-skill" data-skill="${escapeAttr(skill.skillId)}" type="button">
      <div class="skill-token">${escapeHtml(skill.skillId.slice(0, 2).toUpperCase())}</div>
      <div class="pref-main">
        <strong>${escapeHtml(skill.skillId)}</strong>
        <span title="${escapeAttr(skill.path)}">${escapeHtml(shortPath(skill.path))}</span>
      </div>
      <small class="count-label">${enabledAgents.length}/${totalAgents || 0} agents</small>
      <span class="row-chevron">Configure</span>
    </button>
  `;
}

function renderSkillDetailPage(skill, model) {
  return `
    <div class="settings-page">
      <section class="detail-head">
        <button class="mac-button quiet" data-action="back-to-skills" type="button">Back</button>
        <div class="detail-title">
          <div class="skill-token">${escapeHtml(skill.skillId.slice(0, 2).toUpperCase())}</div>
          <div>
            <h2>${escapeHtml(skill.skillId)}</h2>
            <p title="${escapeAttr(skill.path)}">${escapeHtml(shortPath(skill.path))}</p>
          </div>
        </div>
      </section>

      ${environmentGroups(model).map((environment) => renderSkillEnvironment(environment, skill, model)).join("")}
    </div>
  `;
}

function renderSkillEnvironment(environment, skill, model) {
  return renderPreferenceBlock(environmentTitle(environment), environmentSubtitle(environment), `
    ${environment.agents.map((agent) => renderSkillAgentToggle(environment, agent, skill, model)).join("") || renderEmptyState("No agents configured", "Add agent folders for this environment first.")}
  `);
}

function renderSkillAgentToggle(environment, agent, skill, model) {
  const enabled = agent.enabledSkillIds?.includes(skill.skillId) ?? false;
  const rows = model.statuses.filter(
    (item) =>
      item.environmentId === environment.environmentId &&
      item.agentId === agent.agentId &&
      item.skillId === skill.skillId,
  );
  const status = rows[0]?.status ?? (enabled ? "pending" : "disabled");

  return `
    <div class="pref-row skill-agent-row">
      <div class="pref-main">
        <strong>${escapeHtml(agentLabel(agent.agentId))}</strong>
        <span title="${escapeAttr(agent.skillsDir)}">${escapeHtml(shortPath(agent.skillsDir))}</span>
      </div>
      <span class="status-pill ${statusTone(status)}">${escapeHtml(status)}</span>
      ${renderAgentToggle(environment.environmentId, agent.agentId, agentLabel(agent.agentId), skill.skillId, enabled)}
    </div>
  `;
}

function renderAgentsPage(model, state) {
  return `
    <div class="settings-page wide">
      ${renderSectionIntro("Agent folders", "Local and remote agent targets are managed by environment. Remote targets appear after Remote is configured.")}
      <div class="agent-panels">
        ${environmentGroups(model).map((environment) => renderEnvironmentAgentGroup(environment, model, state)).join("")}
      </div>
    </div>
  `;
}

function renderEnvironmentAgentGroup(environment, model, state) {
  return `
    <section class="preference-section">
      <div class="group-heading">
        <h3>${escapeHtml(environmentTitle(environment))}</h3>
        <p>${escapeHtml(environmentSubtitle(environment))}</p>
      </div>
      <div class="agent-environment-list">
        ${AGENTS.map(([agentId, label]) => renderAgentSection(environment, agentId, label, model, state)).join("")}
      </div>
    </section>
  `;
}

function renderAgentSection(environment, agentId, label, model, state) {
  const agent = environment.agents.find((item) => item.agentId === agentId);
  const detected = environment.environmentId === "local"
    ? model.dashboard?.detection?.agents?.find((item) => item.agentId === agentId)
    : null;
  const rows = model.statuses.filter(
    (item) => item.environmentId === environment.environmentId && item.agentId === agentId,
  );
  const linked = rows.filter((item) => item.status === "enabled").length;
  const conflicts = rows.filter((item) => item.status === "conflict" || item.status === "invalid").length;
  const commandStatus = detected?.command?.status ?? "unknown";
  const fallbackDir =
    agent?.skillsDir ??
    detected?.recommendedSkillsDir ??
    defaultRemoteAgentDir(agentId);

  return `
    <section class="pref-group agent-card">
      <form class="agent-preference" data-form="agent-dir" data-environment="${escapeAttr(environment.environmentId)}" data-agent="${agentId}">
        <div class="pref-row agent-head">
          <div class="agent-icon">${escapeHtml(label.slice(0, 1))}</div>
          <div class="pref-main">
            <strong>${label}</strong>
            <span>${linked} linked, ${conflicts} conflicts${environment.environmentId === "local" ? `, command ${escapeHtml(commandStatus)}` : ""}</span>
          </div>
          <label class="mac-check">
            <input type="checkbox" name="managed" ${agent?.managed !== false && agent ? "checked" : ""} />
            Managed
          </label>
        </div>
        <div class="pref-row field-row">
          <label for="${escapeAttr(environment.environmentId)}-${escapeAttr(agentId)}-skills-dir">Skills folder</label>
          <input id="${escapeAttr(environment.environmentId)}-${escapeAttr(agentId)}-skills-dir" name="skillsDir" value="${escapeAttr(fallbackDir ?? "")}" />
          <button class="mac-button" type="submit" ${state.busy ? "disabled" : ""}>Save</button>
        </div>
      </form>
    </section>
  `;
}

function renderRemotePage(model, state) {
  return `
    <div class="settings-page wide">
      ${renderSectionIntro("Remote access", "Configure both directions: this Mac can use a remote source, and remote machines can pull from this Mac's source cache.")}
      <section class="remote-mode-grid">
        ${renderRemoteSourceForm(model, state)}
        ${renderRemoteEnvironmentForm(model, state)}
      </section>
    </div>
  `;
}

function renderRemoteSourceForm(model, state) {
  const remoteSource = model.dashboard?.config?.sourceProfiles?.find(
    (profile) => profile.kind === "remote",
  );

  return `
    <form class="remote-card" data-form="remote-source">
      <div class="mode-card-head">
        <span class="mode-icon">IN</span>
        <div>
          <strong>Use remote skills</strong>
          <small>This Mac pulls a remote source into a local cache.</small>
        </div>
      </div>
      <div class="form-list">
        ${renderTextField("Profile ID", "sourceProfileId", remoteSource?.sourceProfileId ?? "remote-personal")}
        ${renderTextField("Host", "host", remoteSource?.host ?? "", "devbox", true)}
        ${renderTextField("User", "user", remoteSource?.user ?? "", "alice", true)}
        ${renderTextField("Remote source", "remoteSourceRoot", remoteSource?.remoteSourceRoot ?? "", "/home/alice/shared-skills", true)}
        ${renderTextField("Local cache", "localCacheRoot", remoteSource?.localCacheRoot ?? "~/.skills-manager/cache/remote-personal", "", true)}
      </div>
      ${renderOptionStrip(remoteSource)}
      <button class="mac-button primary" type="submit" ${state.busy ? "disabled" : ""}>Save source</button>
    </form>
  `;
}

function renderRemoteEnvironmentForm(model, state) {
  const remoteEnv = model.dashboard?.config?.environments?.find(
    (environment) => environment.kind === "remote",
  );

  return `
    <form class="remote-card" data-form="remote-environment">
      <div class="mode-card-head">
        <span class="mode-icon">OUT</span>
        <div>
          <strong>Remote pulls from this Mac</strong>
          <small>Prepare a remote cache so agents on that machine use this Mac's skills.</small>
        </div>
      </div>
      <div class="form-list">
        ${renderTextField("Environment", "environmentId", remoteEnv?.environmentId ?? "devbox", "", true)}
        ${renderTextField("Host", "host", remoteEnv?.host ?? "", "devbox", true)}
        ${renderTextField("User", "user", remoteEnv?.user ?? "", "alice", true)}
        ${renderTextField("Remote cache", "remoteCacheRoot", remoteEnv?.remoteCacheRoot ?? "~/.skills-manager/cache/personal", "", true)}
        <label class="form-row">
          <span>Direction</span>
          <select name="direction">
            <option value="push-local-to-remote" ${remoteEnv?.syncDirection !== "pull-remote-to-local" ? "selected" : ""}>Push local to remote</option>
            <option value="pull-remote-to-local" ${remoteEnv?.syncDirection === "pull-remote-to-local" ? "selected" : ""}>Pull remote to local</option>
          </select>
        </label>
        ${renderTextField("Codex folder", "codexSkillsDir", remoteAgentDir(remoteEnv, "codex"), "~/.codex/skills")}
        ${renderTextField("Claude folder", "claudeCodeSkillsDir", remoteAgentDir(remoteEnv, "claude-code"), "~/.claude/skills")}
        ${renderTextField("OpenCode folder", "opencodeSkillsDir", remoteAgentDir(remoteEnv, "opencode"), "~/.config/opencode/skills")}
      </div>
      ${renderOptionStrip(remoteEnv)}
      <button class="mac-button primary" type="submit" ${state.busy ? "disabled" : ""}>Save remote</button>
    </form>
  `;
}

function renderSyncPage(model, state) {
  const remoteEnv = model.dashboard?.config?.environments?.find(
    (environment) => environment.kind === "remote",
  );
  const remoteSource = model.dashboard?.config?.sourceProfiles?.find(
    (profile) => profile.kind === "remote",
  );
  const environmentId = remoteEnv?.environmentId ?? "";

  return `
    <div class="settings-page">
      ${renderSectionIntro("Remote sync", "Plan first when you want to inspect the commands. Run applies the same plan.")}
      <section class="sync-console">
        <div class="sync-head">
          <div>
            <strong>Remote target</strong>
            <span>${remoteEnv ? `${escapeHtml(remoteEnv.user)}@${escapeHtml(remoteEnv.host)}` : "Not configured"}</span>
          </div>
          <button class="mac-button" data-action="remote-cli-status" data-environment="${escapeAttr(environmentId)}" ${state.busy || !remoteEnv ? "disabled" : ""}>Test CLI</button>
        </div>
        ${renderSyncAction("Prepare remote cache", remoteEnv?.remoteCacheRoot, "push-local-to-remote", environmentId, Boolean(remoteEnv), state)}
        ${renderSyncAction("Pull remote source", remoteSource?.localCacheRoot, "pull-remote-to-local", "", Boolean(remoteSource), state)}
      </section>
      ${renderRemoteSyncResult(state.lastRemoteSync)}
    </div>
  `;
}

function renderSyncAction(title, path, direction, environmentId, enabled, state) {
  return `
    <div class="sync-row">
      <div class="pref-main">
        <strong>${escapeHtml(title)}</strong>
        <span>${path ? escapeHtml(path) : "Configure Remote first"}</span>
      </div>
      <div class="row-actions">
        <button class="mac-button" data-action="remote-sync-plan" data-direction="${escapeAttr(direction)}" data-environment="${escapeAttr(environmentId)}" ${state.busy || !enabled ? "disabled" : ""}>Plan</button>
        <button class="mac-button primary" data-action="remote-sync-run" data-direction="${escapeAttr(direction)}" data-environment="${escapeAttr(environmentId)}" ${state.busy || !enabled ? "disabled" : ""}>Run</button>
      </div>
    </div>
  `;
}

function renderAdvancedPage(model, state) {
  return `
    <div class="settings-page">
      ${renderPreferenceBlock("OpenCode integration", "OpenCode reads skills.paths directly, so this keeps its native configuration aligned.", `
        <div class="pref-row">
          <div class="pref-main">
            <strong>OpenCode skill path</strong>
            <span>Update OpenCode's native skills.paths setting.</span>
          </div>
          <button class="mac-button" data-action="opencode-path" ${state.busy ? "disabled" : ""}>Update</button>
        </div>
      `)}
      ${renderPreferenceBlock("Hook diagnostics", "Startup hook support detected for each agent.", `
        ${(model.dashboard?.hooks ?? []).map(renderHookRow).join("") || renderEmptyState("No hook information", "Refresh after installing an agent integration.")}
      `)}
    </div>
  `;
}

function renderRemoteSyncResult(result) {
  if (!result) return "";
  const linkActions = result.remoteLink?.actions ?? [];
  const plannedLinks = result.remoteLinkPlan?.links ?? [];
  const applied = linkActions.filter((action) => action.status === "applied").length;
  const skipped = linkActions.filter((action) => action.status === "skipped").length;
  const conflicts = linkActions.filter((action) => action.status === "conflict").length;
  const failed = linkActions.filter((action) => action.status === "failed").length;

  return renderPreferenceBlock("Last remote operation", "Remote cache sync and agent link results.", `
    <div class="pref-row">
      <span class="pref-label">Cache sync</span>
      <strong>${result.sync ? "Completed" : result.plan ? "Planned" : "Not run"}</strong>
    </div>
    <div class="pref-row">
      <span class="pref-label">Remote links</span>
      <strong>${result.remoteLink ? `${applied} applied, ${skipped} skipped, ${conflicts} conflicts, ${failed} failed` : `${plannedLinks.length} planned`}</strong>
    </div>
    ${linkActions.slice(0, 6).map(renderRemoteLinkAction).join("")}
  `);
}

function renderRemoteLinkAction(action) {
  return `
    <div class="pref-row">
      <div class="pref-main">
        <strong>${escapeHtml(agentLabel(action.agentId))} · ${escapeHtml(action.skillId)}</strong>
        <span title="${escapeAttr(action.targetPath)}">${escapeHtml(shortPath(action.targetPath))}</span>
      </div>
      <span class="status-pill ${statusTone(action.status)}">${escapeHtml(action.status)}</span>
    </div>
  `;
}

function renderSetupPage(state) {
  return `
    <div class="setup-sheet">
      <section class="setup-intro">
        <span class="app-glyph large">SM</span>
        <h2>Choose a shared skills source</h2>
        <p>Skills Manager links selected skills into Codex, Claude Code, and OpenCode without copying content between agent folders.</p>
      </section>
      <form class="setup-form" data-form="init">
        ${renderTextField("Shared source", "sourceRoot", "", "/Users/alice/shared-skills", true)}
        ${renderTextField("Codex folder", "codexSkillsDir", "", "~/.codex/skills")}
        ${renderTextField("Claude folder", "claudeCodeSkillsDir", "", "~/.claude/skills")}
        ${renderTextField("OpenCode folder", "opencodeSkillsDir", "", "~/.config/opencode/skills")}
        <button class="mac-button primary" type="submit" ${state.busy ? "disabled" : ""}>Initialize</button>
      </form>
    </div>
  `;
}

function renderAgentToggle(environmentId, agentId, label, skillId, enabled) {
  return `
    <button class="agent-chip ${enabled ? "on" : ""}" data-action="toggle-skill" data-environment="${escapeAttr(environmentId)}" data-agent="${agentId}" data-skill="${escapeAttr(skillId)}" aria-pressed="${enabled}" title="${escapeAttr(label)}" type="button">
      ${enabled ? "On" : "Off"}
    </button>
  `;
}

function renderAgentSummary(agentId, label, model) {
  const localEnvironment = environmentGroups(model).find((environment) => environment.environmentId === "local");
  const agent = localEnvironment?.agents.find((item) => item.agentId === agentId);
  const rows = model.statuses.filter((item) => item.environmentId === "local" && item.agentId === agentId);
  const linked = rows.filter((item) => item.status === "enabled").length;
  const conflicts = rows.filter((item) => item.status === "conflict" || item.status === "invalid").length;
  const managed = agent?.managed !== false && Boolean(agent);

  return `
    <article class="agent-summary">
      <div class="agent-icon">${escapeHtml(label.slice(0, 1))}</div>
      <div>
        <strong>${escapeHtml(label)}</strong>
        <span>${managed ? `${linked} linked` : "Not managed"}</span>
      </div>
      <span class="status-pill ${conflicts ? "bad" : managed ? "good" : "muted"}">${conflicts ? `${conflicts} conflicts` : managed ? "Ready" : "Off"}</span>
    </article>
  `;
}

function renderHookRow(hook) {
  return `
    <div class="pref-row">
      <div class="pref-main">
        <strong>${escapeHtml(hook.agentId)}</strong>
        <span>${escapeHtml(hook.reason)}</span>
      </div>
      <span class="status-pill ${hook.status === "ready" ? "good" : "muted"}">${escapeHtml(hook.status)}</span>
    </div>
  `;
}

function renderSectionIntro(title, text) {
  return `
    <section class="section-intro">
      <h2>${escapeHtml(title)}</h2>
      <p>${escapeHtml(text)}</p>
    </section>
  `;
}

function renderPreferenceBlock(title, text, rows) {
  return `
    <section class="preference-section">
      <div class="group-heading">
        <h3>${escapeHtml(title)}</h3>
        <p>${escapeHtml(text)}</p>
      </div>
      <div class="pref-group">
        ${rows}
      </div>
    </section>
  `;
}

function renderInfoRow(label, value) {
  return `
    <div class="pref-row">
      <span class="pref-label">${escapeHtml(label)}</span>
      <strong class="mono-value" title="${escapeAttr(value ?? "")}">${escapeHtml(value ?? "Not set")}</strong>
    </div>
  `;
}

function renderMetricTile(label, value, helper, tone = "") {
  return `
    <article class="metric-tile ${tone}">
      <span>${escapeHtml(label)}</span>
      <strong>${escapeHtml(value)}</strong>
      <small>${escapeHtml(helper)}</small>
    </article>
  `;
}

function renderEmptyState(title, text) {
  return `
    <div class="empty-state">
      <strong>${escapeHtml(title)}</strong>
      <span>${escapeHtml(text)}</span>
    </div>
  `;
}

function renderTextField(label, name, value, placeholder = "", required = false) {
  return `
    <label class="form-row">
      <span>${escapeHtml(label)}</span>
      <input name="${escapeAttr(name)}" value="${escapeAttr(value ?? "")}" placeholder="${escapeAttr(placeholder)}" ${required ? "required" : ""} />
    </label>
  `;
}

function renderOptionStrip(config) {
  return `
    <div class="option-strip">
      <label class="mac-check">
        <input type="checkbox" name="autoSync" ${config?.autoSync ? "checked" : ""} />
        Auto sync
      </label>
      <label class="mac-check">
        <input type="checkbox" name="deleteExtraneous" ${config?.deleteExtraneous !== false ? "checked" : ""} />
        Delete extra files
      </label>
    </div>
  `;
}

function navMark(id) {
  return {
    general: "G",
    skills: "S",
    agents: "A",
    remote: "R",
    sync: "Y",
    advanced: "X",
  }[id];
}

function pageTitle(page) {
  return {
    general: "General",
    skills: "Skills",
    agents: "Agents",
    remote: "Remote",
    sync: "Sync",
    advanced: "Advanced",
  }[page];
}

function pageSubtitle(page) {
  return {
    general: "Source health, active profile, and linked agent summary.",
    skills: "Choose which agents can see each installed skill.",
    agents: "Manage the skills folder used by each local agent.",
    remote: "Connect remote sources and remote machines from one place.",
    sync: "Plan and run cache synchronization over SSH.",
    advanced: "Agent-specific integrations and startup hook diagnostics.",
  }[page];
}

function remoteAgentDir(remoteEnv, agentId) {
  return remoteEnv?.agents?.find((agent) => agent.agentId === agentId)?.skillsDir ?? "";
}

function environmentGroups(model) {
  return model.dashboard?.config?.environments ?? [];
}

function allConfiguredAgents(model) {
  return environmentGroups(model).flatMap((environment) =>
    (environment.agents ?? []).map((agent) => ({ environment, agent })),
  );
}

function environmentTitle(environment) {
  if (environment.environmentId === "local") return "This Mac";
  return `Remote: ${environment.environmentId}`;
}

function environmentSubtitle(environment) {
  if (environment.environmentId === "local") return "Agents installed on this machine.";
  const remote = environment.user && environment.host ? `${environment.user}@${environment.host}` : "Remote machine";
  return `${remote} pulls from the configured cache.`;
}

function agentLabel(agentId) {
  return AGENTS.find(([id]) => id === agentId)?.[1] ?? agentId;
}

function defaultRemoteAgentDir(agentId) {
  return {
    codex: "~/.codex/skills",
    "claude-code": "~/.claude/skills",
    opencode: "~/.config/opencode/skills",
  }[agentId];
}

function statusTone(status) {
  if (status === "enabled") return "good";
  if (status === "conflict" || status === "invalid") return "bad";
  return "muted";
}
