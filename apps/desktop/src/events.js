import { emptyToNull } from "./format.js";

export function bindView({ action, api, refresh, render, root, state }) {
  bindAction(root, "refresh", refresh);
  bindAction(root, "select-settings-page", (event) => {
    state.settingsPage = event.currentTarget.dataset.page;
    state.selectedSkillId = null;
    render();
  });
  bindAction(root, "select-skill", (event) => {
    state.settingsPage = event.currentTarget.dataset.page || "skills-local";
    state.selectedSkillId = event.currentTarget.dataset.skill;
    render();
  });
  bindAction(root, "back-to-skills", () => {
    state.selectedSkillId = null;
    render();
  });
  bindAction(root, "reconcile-all", () => {
    action(() => api.reconcileAll());
  });
  bindAction(root, "opencode-path", () => {
    action(() => api.updateOpenCodePath());
  });
  bindAction(root, "remote-sync-plan", (event) => {
    const direction = event.currentTarget.dataset.direction;
    const environmentId = event.currentTarget.dataset.environment || null;
    action(() => api.remoteSync({ environmentId, direction, plan: true, repoId: null }));
  });
  bindAction(root, "remote-sync-run", (event) => {
    const direction = event.currentTarget.dataset.direction;
    const environmentId = event.currentTarget.dataset.environment || null;
    action(() => api.remoteSync({ environmentId, direction, plan: false, repoId: null }));
  });
  bindAction(root, "remote-cli-status", (event) => {
    action(() =>
      api.remoteCliStatus({
        environmentId: event.currentTarget.dataset.environment,
      }),
    );
  });

  root.querySelector('[data-form="init"]')?.addEventListener("submit", (event) => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    action(() =>
      api.initConfig({
        sourceRoot: form.get("sourceRoot"),
        codexSkillsDir: emptyToNull(form.get("codexSkillsDir")),
        claudeCodeSkillsDir: emptyToNull(form.get("claudeCodeSkillsDir")),
        opencodeSkillsDir: emptyToNull(form.get("opencodeSkillsDir")),
      }),
    );
  });

  root.querySelectorAll('[data-form="agent-dir"]').forEach((formEl) => {
    formEl.addEventListener("submit", (event) => {
      event.preventDefault();
      const form = new FormData(event.currentTarget);
      action(() =>
        api.setAgentDir({
          environmentId: event.currentTarget.dataset.environment || null,
          agentId: event.currentTarget.dataset.agent,
          skillsDir: form.get("skillsDir"),
          managed: form.get("managed") === "on",
        }),
      );
    });
  });

  root.querySelector('[data-form="local-source"]')?.addEventListener("submit", (event) => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    action(() =>
      api.setLocalSource({
        sourceProfileId: emptyToNull(form.get("sourceProfileId")),
        sourceRoot: form.get("sourceRoot"),
      }),
    );
  });

  root.querySelector('[data-form="remote-source"]')?.addEventListener("submit", (event) => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    action(() =>
      api.setRemoteSource({
        sourceProfileId: emptyToNull(form.get("sourceProfileId")),
        host: form.get("host"),
        user: form.get("user"),
        remoteSourceRoot: form.get("remoteSourceRoot"),
        localCacheRoot: form.get("localCacheRoot"),
        autoSync: form.get("autoSync") === "on",
        deleteExtraneous: form.get("deleteExtraneous") === "on",
      }),
    );
  });

  root.querySelector('[data-form="remote-environment"]')?.addEventListener("submit", (event) => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    action(() =>
      api.setRemoteEnvironment({
        environmentId: form.get("environmentId"),
        host: form.get("host"),
        user: form.get("user"),
        remoteCacheRoot: form.get("remoteCacheRoot"),
        direction: form.get("direction"),
        autoSync: form.get("autoSync") === "on",
        deleteExtraneous: form.get("deleteExtraneous") === "on",
        codexSkillsDir: emptyToNull(form.get("codexSkillsDir")),
        claudeCodeSkillsDir: emptyToNull(form.get("claudeCodeSkillsDir")),
        opencodeSkillsDir: emptyToNull(form.get("opencodeSkillsDir")),
      }),
    );
  });

  root.querySelectorAll('[data-action="toggle-skill"]').forEach((button) => {
    button.addEventListener("click", () => {
      action(() =>
        api.setSkillEnabled({
          environmentId: button.dataset.environment || null,
          agentId: button.dataset.agent,
          skillId: button.dataset.skill,
          enabled: button.getAttribute("aria-pressed") !== "true",
        }),
      );
    });
  });
}

function bindAction(root, actionName, handler) {
  root.querySelectorAll(`[data-action="${actionName}"]`).forEach((button) => {
    button.addEventListener("click", handler);
  });
}
