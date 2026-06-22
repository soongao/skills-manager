import { AGENTS } from "../constants.js";
import { escapeAttr, escapeHtml, shortPath } from "../format.js";

const SETTINGS_NAV = [
  { id: "overview", label: "Overview", hint: "Status" },
  {
    id: "use-local",
    label: "Use Local Skills",
    hint: "This Mac as source",
    children: [
      { id: "local-on-this-mac", label: "On This Mac", hint: "Local agents" },
      { id: "local-on-remote", label: "On Remote Machines", hint: "Remote agents" },
    ],
  },
  {
    id: "use-remote",
    label: "Use Remote Skills",
    hint: "Remote as source",
    children: [
      { id: "remote-source", label: "Remote Source", hint: "Source and cache" },
      { id: "remote-on-this-mac", label: "On This Mac", hint: "Local agents" },
    ],
  },
  {
    id: "skills",
    label: "Skills",
    hint: "Libraries",
    children: [
      { id: "skills-local", label: "Local Library", hint: "Local source skills" },
      { id: "skills-remote", label: "Remote Library", hint: "Pulled remote skills" },
    ],
  },
  { id: "activity", label: "Activity", hint: "Results and conflicts" },
  { id: "advanced", label: "Advanced", hint: "Integrations" },
];

export function renderSettings(model, state) {
  const page = state.settingsPage || "overview";

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
          ${SETTINGS_NAV.map((item) => renderNavItem(item, page)).join("")}
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

function renderNavItem(item, activePage) {
  if (item.children?.length) {
    const active = item.children.some((child) => child.id === activePage);
    return `
      <section class="nav-group ${active ? "open" : ""}">
        <div class="nav-row nav-parent ${active ? "active-parent" : ""}">
          <span class="nav-mark">${navMark(item.id)}</span>
          <span>
            <strong>${escapeHtml(item.label)}</strong>
            <small>${escapeHtml(item.hint)}</small>
          </span>
        </div>
        <div class="nav-children">
          ${item.children.map((child) => renderNavChild(child, activePage)).join("")}
        </div>
      </section>
    `;
  }

  return `
    <button class="nav-row ${activePage === item.id ? "active" : ""}" data-action="select-settings-page" data-page="${item.id}" type="button">
      <span class="nav-mark">${navMark(item.id)}</span>
      <span>
        <strong>${escapeHtml(item.label)}</strong>
        <small>${escapeHtml(item.hint)}</small>
      </span>
    </button>
  `;
}

function renderNavChild(item, activePage) {
  return `
    <button class="nav-child ${activePage === item.id ? "active" : ""}" data-action="select-settings-page" data-page="${item.id}" type="button">
      <strong>${escapeHtml(item.label)}</strong>
      <small>${escapeHtml(item.hint)}</small>
    </button>
  `;
}

function renderPage(page, model, state) {
  switch (page) {
    case "local-on-this-mac":
      return renderLocalOnThisMacPage(model, state);
    case "local-on-remote":
      return renderLocalOnRemotePage(model, state);
    case "remote-source":
      return renderRemoteSourcePage(model, state);
    case "remote-on-this-mac":
      return renderRemoteOnThisMacPage(model, state);
    case "skills-local":
      return renderSkillsPage(model, state, "local");
    case "skills-remote":
      return renderSkillsPage(model, state, "remote");
    case "activity":
      return renderActivityPage(model, state);
    case "advanced":
      return renderAdvancedPage(model, state);
    case "overview":
    default:
      return renderOverviewPage(model);
  }
}

function renderOverviewPage(model) {
  const localEnvironment = localEnvironmentConfig(model);
  const remoteTargets = remoteEnvironments(model);
  const localLibrary = sourceLibrary(model, "local");
  const remoteLibrary = sourceLibrary(model, "remote");

  return `
    <div class="settings-page wide">
      <section class="overview-hero">
        <div class="status-orb ${model.conflictCount ? "bad" : "good"}">
          <span>${model.conflictCount ? "!" : "OK"}</span>
        </div>
        <div class="overview-copy">
          <p class="section-label">Current source</p>
          <h2>${escapeHtml(model.healthLabel)}</h2>
          <p>${model.skills.length} skills are available from ${escapeHtml(shortPath(model.sourceRoot))}. Configure sources and targets first, then apply links from the Skills library.</p>
        </div>
        <span class="status-pill ${model.conflictCount ? "bad" : "good"}">${model.conflictCount ? `${model.conflictCount} conflicts` : "Ready"}</span>
      </section>

      <section class="metric-grid four">
        ${renderMetricTile("Local library", localLibrary.skills.length, "Skills on this Mac")}
        ${renderMetricTile("Remote library", remoteLibrary.skills.length, "Pulled into local cache")}
        ${renderMetricTile("Local agents", localEnvironment?.agents?.length ?? 0, "On this Mac")}
        ${renderMetricTile("Remote machines", remoteTargets.length, "Configured over SSH")}
      </section>

      <section class="workflow-list">
        ${renderWorkflowCard("1", "Use local skills on this Mac", "Pick the local skills folder and the local agent folders that should receive links.", "Configure", "local-on-this-mac")}
        ${renderWorkflowCard("2", "Use local skills on remote machines", "Add SSH targets and their agent folders. Syncing is run from the local library.", "Configure", "local-on-remote")}
        ${renderWorkflowCard("3", "Use remote skills on this Mac", "Set the remote source and local cache, then apply the pulled library to local agents.", "Configure", "remote-source")}
      </section>

      ${renderPreferenceBlock("Current source", "This folder is used when links are created.", `
        ${renderInfoRow("Source folder", model.sourceRoot)}
        ${renderInfoRow("Config home", model.dashboard?.configHome)}
        ${renderInfoRow("Active profile", model.dashboard?.config?.activeSourceProfileId)}
      `)}
    </div>
  `;
}

function renderWorkflowCard(index, title, text, actionLabel, page) {
  return `
    <button class="workflow-card" data-action="select-settings-page" data-page="${escapeAttr(page)}" type="button">
      <span class="workflow-index">${escapeHtml(index)}</span>
      <strong>${escapeHtml(title)}</strong>
      <small>${escapeHtml(text)}</small>
      <span>${escapeHtml(actionLabel)}</span>
    </button>
  `;
}

function renderLocalOnThisMacPage(model, state) {
  return `
    <div class="settings-page wide">
      ${renderSectionIntro("On This Mac", "Configure this Mac to read local skills and expose them to local agents. Applying skills is run from Skills.")}
      ${renderLocalSourceForm(model, state)}
      ${renderLocalTargetsPanel(
        model,
        state,
        "Local agents",
        "Configure the local agent skills folders. These settings are used when you apply skills from the Skills page.",
      )}
    </div>
  `;
}

function renderLocalOnRemotePage(model, state) {
  return `
    <div class="settings-page wide">
      ${renderSectionIntro("On Remote Machines", "Configure remote machines that should use this Mac's local source. Syncing and applying skills are run from Skills.")}
      ${renderLocalSourceForm(model, state)}
      ${renderRemoteEnvironmentForm(model, state)}
      ${renderRemoteTargetsPanel(model, state)}
    </div>
  `;
}

function renderRemoteSourcePage(model, state) {
  return `
    <div class="settings-page wide">
      ${renderSectionIntro("Remote Source", "Configure the SSH source and the local cache that will hold pulled remote skills. Pulling is run from Skills.")}
      ${renderRemoteSourceForm(model, state)}
    </div>
  `;
}

function renderRemoteOnThisMacPage(model, state) {
  return `
    <div class="settings-page wide">
      ${renderSectionIntro("On This Mac", "Configure local agent folders that will use the pulled remote cache. Applying skills is run from Skills.")}
      ${renderLocalTargetsPanel(
        model,
        state,
        "Local agents using remote cache",
        "Configure the local agent folders that will use the pulled cache.",
      )}
    </div>
  `;
}

function renderLocalSourceForm(model, state) {
  const localSource = sourceProfileByKind(model, "local");
  const isActive = model.dashboard?.config?.activeSourceProfileId === localSource?.sourceProfileId;

  return `
    <form class="remote-card source-card" data-form="local-source">
      <div class="mode-card-head">
        <span class="mode-icon">SRC</span>
        <div>
          <strong>Local source</strong>
          <small>The local folder this Mac reads from.</small>
        </div>
        <span class="status-pill ${isActive ? "good" : "muted"}">${isActive ? "Active" : "Saved"}</span>
      </div>
      <div class="form-list">
        ${renderTextField("Profile ID", "sourceProfileId", localSource?.sourceProfileId ?? "local-personal")}
        ${renderTextField("Source folder", "sourceRoot", localSource?.sourceRoot ?? model.sourceRoot, "/Users/alice/shared-skills", true)}
      </div>
      <button class="mac-button primary" type="submit" ${state.busy ? "disabled" : ""}>Save local source</button>
    </form>
  `;
}

function renderRemoteSourceForm(model, state) {
  const remoteSource = sourceProfileByKind(model, "remote");
  const isActive = model.dashboard?.config?.activeSourceProfileId === remoteSource?.sourceProfileId;

  return `
    <form class="remote-card source-card" data-form="remote-source">
      <div class="mode-card-head">
        <span class="mode-icon">SRC</span>
        <div>
          <strong>Remote source</strong>
          <small>Pull a remote skills source into this Mac's local cache.</small>
        </div>
        <span class="status-pill ${isActive ? "good" : remoteSource ? "muted" : "bad"}">${isActive ? "Active" : remoteSource ? "Saved" : "Not set"}</span>
      </div>
      <div class="form-list">
        ${renderTextField("Profile ID", "sourceProfileId", remoteSource?.sourceProfileId ?? "remote-personal")}
        ${renderTextField("Host", "host", remoteSource?.host ?? "", "devbox", true)}
        ${renderTextField("User", "user", remoteSource?.user ?? "", "alice", true)}
        ${renderTextField("Remote source", "remoteSourceRoot", remoteSource?.remoteSourceRoot ?? "", "/home/alice/shared-skills", true)}
        ${renderTextField("Local cache", "localCacheRoot", remoteSource?.localCacheRoot ?? "~/.skills-manager/cache/remote-personal", "", true)}
      </div>
      ${renderOptionStrip(remoteSource)}
      <button class="mac-button primary" type="submit" ${state.busy ? "disabled" : ""}>Save remote source</button>
    </form>
  `;
}

function renderLocalTargetsPanel(model, state, title, text) {
  const localEnvironment = localEnvironmentConfig(model);
  if (!localEnvironment) {
    return renderPreferenceBlock(title, "Local agents are not configured yet.", renderEmptyState("No local target", "Initialize a source first."));
  }

  return `
    <section class="target-block">
      <div class="target-block-head">
        <div>
          <h3>${escapeHtml(title)}</h3>
          <p>${escapeHtml(text)}</p>
        </div>
      </div>
      <div class="agent-environment-list">
        ${AGENTS.map(([agentId, label]) => renderAgentSection(localEnvironment, agentId, label, model, state)).join("")}
      </div>
    </section>
  `;
}

function renderRemoteEnvironmentForm(model, state) {
  const nextRemoteIndex = remoteEnvironments(model).length + 1;
  const defaultEnvironmentId = nextRemoteIndex === 1 ? "devbox" : `devbox-${nextRemoteIndex}`;

  return `
    <form class="remote-card target-config-card" data-form="remote-environment">
      <div class="mode-card-head">
        <span class="mode-icon">SSH</span>
        <div>
          <strong>Add remote machine</strong>
          <small>Configure the remote cache and remote agent skills folders.</small>
        </div>
      </div>
      <div class="form-list">
        ${renderTextField("Environment", "environmentId", defaultEnvironmentId, "", true)}
        ${renderTextField("Host", "host", "", "devbox", true)}
        ${renderTextField("User", "user", "", "alice", true)}
        ${renderTextField("Remote cache", "remoteCacheRoot", "~/.skills-manager/cache/personal", "", true)}
        <label class="form-row">
          <span>Mode</span>
          <select name="direction">
            <option value="push-local-to-remote" selected>Use this Mac source on remote target</option>
          </select>
        </label>
        ${renderTextField("Codex folder", "codexSkillsDir", "", "~/.codex/skills")}
        ${renderTextField("Claude folder", "claudeCodeSkillsDir", "", "~/.claude/skills")}
        ${renderTextField("OpenCode folder", "opencodeSkillsDir", "", "~/.config/opencode/skills")}
      </div>
      ${renderOptionStrip(null)}
      <button class="mac-button primary" type="submit" ${state.busy ? "disabled" : ""}>Add remote machine</button>
    </form>
  `;
}

function renderRemoteTargetsPanel(model, state) {
  const remoteEnvs = remoteEnvironments(model);

  return renderPreferenceBlock("Remote machines", "Configured remote machines. Sync and linking are run from Skills.", `
    ${remoteEnvs.map((environment) => renderRemoteTargetEnvironment(environment, model, state)).join("") || renderEmptyState("No remote machines", "Add a remote machine above.")}
  `);
}

function renderRemoteTargetEnvironment(environment, model, state) {
  return `
    <section class="remote-target-section compact-section">
      <div class="remote-target-title">
        <div>
          <strong>${escapeHtml(environmentTitle(environment))}</strong>
          <span>${escapeHtml(environmentSubtitle(environment))}</span>
        </div>
        <div class="row-actions">
          <button class="mac-button" data-action="remote-cli-status" data-environment="${escapeAttr(environment.environmentId)}" ${state.busy ? "disabled" : ""}>Test SSH</button>
        </div>
      </div>
      <div class="agent-environment-list">
        ${AGENTS.map(([agentId, label]) => renderAgentSection(environment, agentId, label, model, state)).join("")}
      </div>
    </section>
  `;
}

function renderAgentSection(environment, agentId, label, model, state) {
  const agent = environment.agents.find((item) => item.agentId === agentId);
  const detected =
    environment.environmentId === "local"
      ? model.dashboard?.detection?.agents?.find((item) => item.agentId === agentId)
      : null;
  const rows = model.statuses.filter(
    (item) => item.environmentId === environment.environmentId && item.agentId === agentId,
  );
  const linked = rows.filter((item) => item.status === "enabled").length;
  const conflicts = rows.filter((item) => item.status === "conflict" || item.status === "invalid").length;
  const commandStatus = detected?.command?.status ?? "unknown";
  const fallbackDir = agent?.skillsDir ?? detected?.recommendedSkillsDir ?? defaultRemoteAgentDir(agentId);

  return `
    <section class="pref-group agent-card">
      <form class="agent-preference" data-form="agent-dir" data-environment="${escapeAttr(environment.environmentId)}" data-agent="${agentId}">
        <div class="pref-row agent-head">
          <div class="agent-icon">${escapeHtml(label.slice(0, 1))}</div>
          <div class="pref-main">
            <strong>${escapeHtml(label)}</strong>
            <span>${linked} enabled, ${conflicts} conflicts${environment.environmentId === "local" ? `, command ${escapeHtml(commandStatus)}` : ""}</span>
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

function renderSkillsPage(model, state, sourceKind = "local") {
  const library = sourceLibrary(model, sourceKind);
  const selectedSkill = library.skills.find((skill) => skill.skillId === state.selectedSkillId);
  if (selectedSkill) return renderSkillDetailPage(selectedSkill, model, sourceKind);
  const title = sourceKind === "remote" ? "Remote Library" : "Local Library";
  const intro =
    sourceKind === "remote"
      ? "Review skills pulled from a remote source, pull the latest cache, and apply them to this Mac's agents."
      : "Review local skills, apply them to this Mac, or sync them to configured remote machines.";
  const emptyText =
    sourceKind === "remote"
      ? "Configure a remote source and pull it into the local cache first."
      : "Choose a shared local source folder that contains skill directories.";

  return `
    <div class="settings-page wide">
      ${renderSectionIntro(title, intro)}
      ${renderLibrarySummary(library)}
      ${renderSkillApplyPanel(model, state, sourceKind)}
      <section class="pref-group skill-list">
        ${library.skills.map((skill) => renderSkillPreference(skill, model, sourceKind)).join("") || renderEmptyState("No skills found", emptyText)}
      </section>
    </div>
  `;
}

function renderLibrarySummary(library) {
  const helper = library.error
    ? "This source cannot be scanned yet."
    : library.active
      ? "This is the source used by apply and sync actions."
      : "This source is saved, but apply and sync actions require it to be active.";

  return renderPreferenceBlock("Library source", helper, `
    ${renderInfoRow("Profile", library.sourceProfileId)}
    ${renderInfoRow("Folder", library.sourceRoot)}
    ${renderInfoRow("Active", library.active ? "Yes" : "No")}
    ${library.error ? renderInfoRow("Scan status", library.error) : renderInfoRow("Skills", String(library.skills.length))}
  `);
}

function renderSkillApplyPanel(model, state, sourceKind) {
  const localReady = Boolean(localEnvironmentConfig(model));
  const remoteSource = sourceProfileByKind(model, "remote");
  const activeKind = activeSourceKind(model);
  const remoteEnvs = remoteEnvironments(model);
  const localActive = activeKind === "local";
  const remoteActive = activeKind === "remote";

  if (sourceKind === "remote") {
    return renderPreferenceBlock("Run remote library actions", "Pull the remote source first, then apply the cached skills to local agents.", `
      <div class="pref-row action-row">
        <div class="pref-main">
          <strong>Pull remote source</strong>
          <span>Update this Mac's local cache from the configured remote skills source.</span>
        </div>
        <div class="row-actions">
          <button class="mac-button" data-action="remote-sync-plan" data-direction="pull-remote-to-local" data-environment="" ${state.busy || !remoteSource || !remoteActive ? "disabled" : ""}>Plan pull</button>
          <button class="mac-button primary" data-action="remote-sync-run" data-direction="pull-remote-to-local" data-environment="" ${state.busy || !remoteSource || !remoteActive ? "disabled" : ""}>Pull to this Mac</button>
        </div>
      </div>
      <div class="pref-row action-row">
        <div class="pref-main">
          <strong>Local agents</strong>
          <span>Apply the pulled cache to Codex, Claude Code, and OpenCode on this Mac.</span>
        </div>
        <button class="mac-button primary" data-action="reconcile-all" ${state.busy || !localReady || !remoteActive ? "disabled" : ""}>Apply local links</button>
      </div>
    `);
  }

  return renderPreferenceBlock("Run local library actions", "Apply this Mac's local skills to local agents or configured remote machines.", `
    <div class="pref-row action-row">
      <div class="pref-main">
        <strong>Local agents</strong>
        <span>Apply the current source to Codex, Claude Code, and OpenCode on this Mac.</span>
      </div>
      <button class="mac-button primary" data-action="reconcile-all" ${state.busy || !localReady || !localActive ? "disabled" : ""}>Apply local links</button>
    </div>
    ${remoteEnvs.map((environment) => renderRemoteApplyRow(environment, state, localActive)).join("") || renderEmptyState("No remote machines", "Configure remote machines before syncing remote agents.")}
  `);
}

function renderRemoteApplyRow(environment, state, enabled) {
  const remote = environment.user && environment.host ? `${environment.user}@${environment.host}` : environment.environmentId;

  return `
    <div class="pref-row action-row">
      <div class="pref-main">
        <strong>${escapeHtml(environment.environmentId)}</strong>
        <span>Sync this Mac's local skills to ${escapeHtml(remote)}, then apply links inside the remote agent folders.</span>
      </div>
      <div class="row-actions">
        <button class="mac-button" data-action="remote-sync-plan" data-direction="push-local-to-remote" data-environment="${escapeAttr(environment.environmentId)}" ${state.busy || !enabled ? "disabled" : ""}>Plan sync</button>
        <button class="mac-button primary" data-action="remote-sync-run" data-direction="push-local-to-remote" data-environment="${escapeAttr(environment.environmentId)}" ${state.busy || !enabled ? "disabled" : ""}>Sync & apply</button>
      </div>
    </div>
  `;
}

function renderSkillPreference(skill, model, sourceKind) {
  const targets = skillTargetAgents(model, sourceKind);
  const enabledAgents = targets.filter(({ agent }) =>
    agent.enabledSkillIds?.includes(skill.skillId),
  );
  const page = sourceKind === "remote" ? "skills-remote" : "skills-local";

  return `
    <button class="pref-row skill-pref skill-open-row" data-action="select-skill" data-page="${page}" data-skill="${escapeAttr(skill.skillId)}" type="button">
      <div class="skill-token">${escapeHtml(skill.skillId.slice(0, 2).toUpperCase())}</div>
      <div class="pref-main">
        <strong>${escapeHtml(skill.skillId)}</strong>
        <span title="${escapeAttr(skill.path)}">${escapeHtml(shortPath(skill.path))}</span>
      </div>
      <small class="count-label">${enabledAgents.length}/${targets.length || 0} targets</small>
      <span class="row-chevron">Open</span>
    </button>
  `;
}

function renderSkillDetailPage(skill, model, sourceKind) {
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

      ${skillTargetEnvironments(model, sourceKind).map((environment) => renderSkillEnvironment(environment, skill, model, sourceKind)).join("")}
    </div>
  `;
}

function renderSkillEnvironment(environment, skill, model, sourceKind) {
  return renderPreferenceBlock(environmentTitle(environment), environmentSubtitle(environment), `
    ${environment.agents.map((agent) => renderSkillAgentToggle(environment, agent, skill, model, sourceKind)).join("") || renderEmptyState("No agents configured", "Add agent folders for this environment first.")}
  `);
}

function renderSkillAgentToggle(environment, agent, skill, model, sourceKind) {
  const enabled = agent.enabledSkillIds?.includes(skill.skillId) ?? false;
  const libraryActive = activeSourceKind(model) === sourceKind;
  const rows = model.statuses.filter(
    (item) =>
      item.environmentId === environment.environmentId &&
      item.agentId === agent.agentId &&
      item.skillId === skill.skillId,
  );
  const status = libraryActive ? rows[0]?.status ?? (enabled ? "pending" : "disabled") : enabled ? "enabled" : "disabled";

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

function renderActivityPage(model, state) {
  const problemRows = model.statuses.filter((item) => item.status === "conflict" || item.status === "invalid");

  return `
    <div class="settings-page">
      ${renderSectionIntro("Activity", "Review the last operation and any skipped conflicts. Conflicts are shown, never overwritten.")}
      ${renderRemoteSyncResult(state.lastRemoteSync)}
      ${renderConflictPanel(problemRows)}
    </div>
  `;
}

function renderConflictPanel(problemRows) {
  return renderPreferenceBlock("Current conflicts", "Resolve these paths manually, then apply links again.", `
    ${problemRows.map(renderStatusProblemRow).join("") || renderEmptyState("No conflicts", "All configured links are clear.")}
  `);
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
  if (!result) {
    return renderPreferenceBlock("Last operation", "No remote operation has run in this window.", renderEmptyState("No activity yet", "Run a source pull or remote sync to see results here."));
  }

  const linkActions = result.remoteLink?.actions ?? [];
  const plannedLinks = result.remoteLinkPlan?.links ?? [];
  const applied = linkActions.filter((action) => action.status === "applied").length;
  const skipped = linkActions.filter((action) => action.status === "skipped").length;
  const conflicts = linkActions.filter((action) => action.status === "conflict").length;
  const failed = linkActions.filter((action) => action.status === "failed").length;
  const direction = result.sync?.plan?.direction ?? result.plan?.direction ?? "unknown";
  const operationLabel =
    direction === "pull-remote-to-local"
      ? "Remote source pull"
      : direction === "push-local-to-remote"
        ? "Remote machine sync"
        : "Remote operation";

  return renderPreferenceBlock("Last operation", operationLabel, `
    <div class="pref-row">
      <span class="pref-label">Cache sync</span>
      <strong>${result.sync ? "Completed" : result.plan ? "Planned" : "Not run"}</strong>
    </div>
    <div class="pref-row">
      <span class="pref-label">Remote links</span>
      <strong>${result.remoteLink ? `${applied} applied, ${skipped} skipped, ${conflicts} conflicts, ${failed} failed` : `${plannedLinks.length} planned`}</strong>
    </div>
    ${linkActions.slice(0, 8).map(renderRemoteLinkAction).join("")}
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

function renderStatusProblemRow(row) {
  return `
    <div class="pref-row">
      <div class="pref-main">
        <strong>${escapeHtml(agentLabel(row.agentId))} · ${escapeHtml(row.skillId)}</strong>
        <span title="${escapeAttr(row.targetPath)}">${escapeHtml(shortPath(row.targetPath))}</span>
      </div>
      <span class="status-pill ${statusTone(row.status)}">${escapeHtml(row.status)}</span>
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
    overview: "O",
    "use-local": "L",
    "use-remote": "R",
    skills: "K",
    activity: "A",
    advanced: "X",
  }[id];
}

function pageTitle(page) {
  return {
    overview: "Overview",
    "local-on-this-mac": "On This Mac",
    "local-on-remote": "On Remote Machines",
    "remote-source": "Remote Source",
    "remote-on-this-mac": "On This Mac",
    "skills-local": "Local Library",
    "skills-remote": "Remote Library",
    activity: "Activity",
    advanced: "Advanced",
  }[page];
}

function pageSubtitle(page) {
  return {
    overview: "Choose the source and target workflow you want to configure.",
    "local-on-this-mac": "Use the local skills folder with agents installed on this Mac.",
    "local-on-remote": "Use the local skills folder with agents installed on remote machines.",
    "remote-source": "Pull a remote skills folder into a local cache.",
    "remote-on-this-mac": "Use pulled remote skills with agents installed on this Mac.",
    "skills-local": "Apply or sync skills from the local library.",
    "skills-remote": "Pull and apply skills from the remote library cache.",
    activity: "Remote operations, skipped conflicts, and exact affected paths.",
    advanced: "Agent-specific integrations and startup hook diagnostics.",
  }[page];
}

function activeSourceKind(model) {
  const activeId = model.dashboard?.config?.activeSourceProfileId;
  return sourceProfiles(model).find((profile) => profile.sourceProfileId === activeId)?.kind;
}

function localEnvironmentConfig(model) {
  return environmentGroups(model).find((environment) => environment.environmentId === "local");
}

function environmentGroups(model) {
  return model.dashboard?.config?.environments ?? [];
}

function remoteEnvironments(model) {
  return environmentGroups(model).filter((environment) => environment.kind === "remote");
}

function sourceProfiles(model) {
  return model.dashboard?.config?.sourceProfiles ?? [];
}

function sourceProfileByKind(model, sourceKind) {
  const activeId = model.dashboard?.config?.activeSourceProfileId;
  const profiles = sourceProfiles(model).filter((profile) => profile.kind === sourceKind);
  return profiles.find((profile) => profile.sourceProfileId === activeId) ?? profiles[0];
}

function sourceLibrary(model, sourceKind) {
  const profile = sourceProfileByKind(model, sourceKind);
  const sourceSkills = model.dashboard?.sourceSkills ?? [];
  const fromDashboard =
    sourceSkills.find((source) => source.sourceProfileId === profile?.sourceProfileId) ??
    sourceSkills.find((source) => source.kind === sourceKind);
  if (fromDashboard) {
    return {
      active: activeSourceKind(model) === sourceKind,
      error: fromDashboard.error ?? null,
      kind: sourceKind,
      skills: fromDashboard.skills ?? [],
      sourceProfileId: fromDashboard.sourceProfileId,
      sourceRoot: fromDashboard.sourceRoot,
    };
  }

  const sourceRoot =
    sourceKind === "remote"
      ? profile?.localCacheRoot
      : profile?.sourceRoot ?? model.sourceRoot;

  return {
    active: activeSourceKind(model) === sourceKind,
    error: profile ? null : "Not configured",
    kind: sourceKind,
    skills: activeSourceKind(model) === sourceKind ? model.skills : [],
    sourceProfileId: profile?.sourceProfileId ?? "Not configured",
    sourceRoot: sourceRoot ?? "",
  };
}

function skillTargetEnvironments(model, sourceKind) {
  const environments = environmentGroups(model);
  if (sourceKind === "remote") {
    return environments.filter((environment) => environment.environmentId === "local");
  }
  return environments;
}

function skillTargetAgents(model, sourceKind) {
  return skillTargetEnvironments(model, sourceKind).flatMap((environment) =>
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
  return `${remote} uses links created inside its configured agent skills folders.`;
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
  if (status === "enabled" || status === "applied") return "good";
  if (status === "conflict" || status === "invalid" || status === "failed") return "bad";
  return "muted";
}
