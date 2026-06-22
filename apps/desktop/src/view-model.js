export function createViewModel(dashboard) {
  const config = dashboard?.config;
  const localEnv = config?.environments?.find((env) => env.environmentId === "local");
  const statuses = dashboard?.statuses ?? [];
  const conflictCount = statuses.filter((item) => isProblemStatus(item.status)).length;
  const enabledCount = statuses.filter((item) => item.status === "enabled").length;

  return {
    agents: localEnv?.agents ?? [],
    configured: Boolean(config),
    conflictCount,
    dashboard,
    enabledCount,
    healthLabel: conflictCount ? "Needs attention" : "All quiet",
    skills: dashboard?.skills ?? [],
    sourceRoot: dashboard?.sourceRoot ?? "",
    statuses,
  };
}

export function isProblemStatus(status) {
  return status === "conflict" || status === "invalid";
}
