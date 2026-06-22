import { invoke } from "@tauri-apps/api/core";
import "./styles.css";

const AGENTS = [
  ["codex", "Codex"],
  ["claude-code", "Claude Code"],
  ["opencode", "OpenCode"],
];

const state = {
  dashboard: null,
  selectedAgent: "claude-code",
  busy: false,
  error: null,
};

const app = document.querySelector("#app");

async function refresh() {
  state.busy = true;
  state.error = null;
  render();
  try {
    state.dashboard = await invoke("load_dashboard", {});
  } catch (error) {
    state.error = normalizeError(error);
  } finally {
    state.busy = false;
    render();
  }
}

async function action(fn) {
  state.busy = true;
  state.error = null;
  render();
  try {
    await fn();
    state.dashboard = await invoke("load_dashboard", {});
  } catch (error) {
    state.error = normalizeError(error);
  } finally {
    state.busy = false;
    render();
  }
}

function normalizeError(error) {
  if (typeof error === "string") return error;
  if (error?.message) return error.message;
  return JSON.stringify(error);
}

function render() {
  const dashboard = state.dashboard;
  const config = dashboard?.config;
  const localEnv = config?.environments?.find((env) => env.environmentId === "local");
  const skills = dashboard?.skills ?? [];
  const statuses = dashboard?.statuses ?? [];
  const agents = localEnv?.agents ?? [];
  const activeSource = dashboard?.sourceRoot ?? "";

  app.innerHTML = `
    <section class="shell">
      <aside class="rail">
        <div class="brand">
          <span class="brand-mark">SM</span>
          <div>
            <strong>Skills Manager</strong>
            <span>shared agent skills</span>
          </div>
        </div>
        <nav>
          <a href="#source">Source</a>
          <a href="#skills">Skills</a>
          <a href="#agents">Agents</a>
          <a href="#sync">Sync</a>
          <a href="#hooks">Hooks</a>
        </nav>
      </aside>
      <section class="workspace">
        <header class="topbar">
          <div>
            <p class="eyebrow">Active source</p>
            <h1>${activeSource || "No source configured"}</h1>
          </div>
          <button class="icon-text" data-action="refresh" ${state.busy ? "disabled" : ""}>Refresh</button>
        </header>
        ${state.error ? `<div class="issue">${escapeHtml(state.error)}</div>` : ""}
        ${!config ? renderSetup() : renderDashboard(dashboard, skills, statuses, agents)}
      </section>
    </section>
  `;

  bind();
}

function renderSetup() {
  return `
    <section id="source" class="band setup-band">
      <div class="section-head">
        <p class="eyebrow">Setup</p>
        <h2>Choose one source folder</h2>
      </div>
      <form class="setup-form" data-form="init">
        <label>
          Shared source root
          <input name="sourceRoot" placeholder="/Users/alice/shared-skills" required />
        </label>
        <label>
          Codex skills dir
          <input name="codexSkillsDir" placeholder="$CODEX_HOME/skills or ~/.agents/skills" />
        </label>
        <label>
          Claude Code skills dir
          <input name="claudeCodeSkillsDir" placeholder="~/.claude/skills" />
        </label>
        <label>
          OpenCode skills dir
          <input name="opencodeSkillsDir" placeholder="~/.config/opencode/skills" />
        </label>
        <button type="submit">Initialize</button>
      </form>
    </section>
  `;
}

function renderDashboard(dashboard, skills, statuses, agents) {
  return `
    <section id="source" class="band source-band">
      <div class="section-head">
        <p class="eyebrow">Source</p>
        <h2>${skills.length} skills discovered</h2>
      </div>
      <div class="source-grid">
        <div>
          <span class="metric">${dashboard.config.activeSourceProfileId}</span>
          <p>Active profile</p>
        </div>
        <div>
          <span class="metric">${dashboard.detection?.ssh?.status ?? "unknown"}</span>
          <p>SSH</p>
        </div>
        <div>
          <span class="metric">${dashboard.detection?.rsync?.status ?? "unknown"}</span>
          <p>rsync</p>
        </div>
      </div>
    </section>
    <section id="skills" class="band">
      <div class="section-head with-actions">
        <div>
          <p class="eyebrow">Skills</p>
          <h2>Enable matrix</h2>
        </div>
        <button data-action="reconcile-all" ${state.busy ? "disabled" : ""}>Apply links</button>
      </div>
      <div class="skill-table">
        <div class="skill-row skill-head">
          <span>Skill</span>
          ${AGENTS.map(([, label]) => `<span>${label}</span>`).join("")}
        </div>
        ${skills.map((skill) => renderSkillRow(skill, agents, statuses)).join("")}
      </div>
    </section>
    <section id="agents" class="band">
      <div class="section-head">
        <p class="eyebrow">Agents</p>
        <h2>Reference directories</h2>
      </div>
      <div class="agent-list">
        ${AGENTS.map(([agentId, label]) => renderAgent(agentId, label, agents, dashboard.detection)).join("")}
      </div>
    </section>
    <section id="sync" class="band">
      <div class="section-head">
        <p class="eyebrow">Sync</p>
        <h2>Remote cache mode</h2>
      </div>
      <div class="empty-line">Remote environments are configured through the CLI in this MVP. The desktop reads the same config and state.</div>
    </section>
    <section id="hooks" class="band">
      <div class="section-head">
        <p class="eyebrow">Hooks</p>
        <h2>Install status</h2>
      </div>
      <div class="hook-grid">
        ${(dashboard.hooks ?? []).map(renderHook).join("")}
      </div>
    </section>
  `;
}

function renderSkillRow(skill, agents, statuses) {
  return `
    <div class="skill-row">
      <span class="skill-name">${escapeHtml(skill.skillId)}</span>
      ${AGENTS.map(([agentId]) => {
        const agent = agents.find((item) => item.agentId === agentId);
        const enabled = agent?.enabledSkillIds?.includes(skill.skillId) ?? false;
        const status = statuses.find((item) => item.agentId === agentId && item.skillId === skill.skillId)?.status ?? "disabled";
        return `
          <span class="toggle-cell">
            <button class="toggle ${enabled ? "on" : ""}" data-action="toggle-skill" data-agent="${agentId}" data-skill="${escapeAttr(skill.skillId)}" aria-pressed="${enabled}">
              ${enabled ? "On" : "Off"}
            </button>
            <small class="status ${status}">${status}</small>
          </span>
        `;
      }).join("")}
    </div>
  `;
}

function renderAgent(agentId, label, agents, detection) {
  const agent = agents.find((item) => item.agentId === agentId);
  const detected = detection?.agents?.find((item) => item.agentId === agentId);
  return `
    <form class="agent-item" data-form="agent-dir" data-agent="${agentId}">
      <div>
        <strong>${label}</strong>
        <span>${detected?.command?.status ?? "unknown"}</span>
      </div>
      <input name="skillsDir" value="${escapeAttr(agent?.skillsDir ?? detected?.recommendedSkillsDir ?? "")}" />
      <label class="check">
        <input type="checkbox" name="managed" ${agent?.managed !== false ? "checked" : ""} />
        Managed
      </label>
      <button type="submit">Save</button>
    </form>
  `;
}

function renderHook(hook) {
  return `
    <div class="hook-card">
      <strong>${hook.agentId}</strong>
      <span>${hook.status}</span>
      <p>${escapeHtml(hook.reason)}</p>
    </div>
  `;
}

function bind() {
  document.querySelector('[data-action="refresh"]')?.addEventListener("click", refresh);
  document.querySelector('[data-action="reconcile-all"]')?.addEventListener("click", () => {
    action(() => invoke("reconcile", { agentId: null, plan: false }));
  });

  document.querySelector('[data-form="init"]')?.addEventListener("submit", (event) => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    action(() =>
      invoke("init_config", {
        sourceRoot: form.get("sourceRoot"),
        codexSkillsDir: emptyToNull(form.get("codexSkillsDir")),
        claudeCodeSkillsDir: emptyToNull(form.get("claudeCodeSkillsDir")),
        opencodeSkillsDir: emptyToNull(form.get("opencodeSkillsDir")),
      }),
    );
  });

  document.querySelectorAll('[data-form="agent-dir"]').forEach((formEl) => {
    formEl.addEventListener("submit", (event) => {
      event.preventDefault();
      const form = new FormData(event.currentTarget);
      action(() =>
        invoke("set_agent_dir", {
          agentId: event.currentTarget.dataset.agent,
          skillsDir: form.get("skillsDir"),
          managed: form.get("managed") === "on",
        }),
      );
    });
  });

  document.querySelectorAll('[data-action="toggle-skill"]').forEach((button) => {
    button.addEventListener("click", () => {
      action(() =>
        invoke("set_skill_enabled", {
          agentId: button.dataset.agent,
          skillId: button.dataset.skill,
          enabled: button.getAttribute("aria-pressed") !== "true",
        }),
      );
    });
  });
}

function emptyToNull(value) {
  const text = String(value ?? "").trim();
  return text ? text : null;
}

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function escapeAttr(value) {
  return escapeHtml(value).replaceAll("'", "&#039;");
}

render();
refresh();
