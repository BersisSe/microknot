import type { Workflow, WorkflowInput, KnotDescriptor, RunResult } from "./types";

const BASE = "/api";

class ApiError extends Error {}

async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
  const res = await fetch(`${BASE}${path}`, options);
  const isJson = (res.headers.get("content-type") || "").includes("application/json");
  const body = isJson ? await res.json() : null;
  if (!res.ok) {
    const msg = body && body.error ? body.error : `Request failed (${res.status})`;
    throw new ApiError(msg);
  }
  return body as T;
}

export const api = {
  listWorkflows(): Promise<{ data: Workflow[] }> {
    return request("/workflows");
  },

  getWorkflow(id: string): Promise<{ data: Workflow }> {
    return request(`/workflows/${id}`);
  },

  createWorkflow(input: WorkflowInput): Promise<{ data: Workflow }> {
    return request("/workflows", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(input),
    });
  },

  updateWorkflow(id: string, input: WorkflowInput): Promise<{ data: Workflow }> {
    return request(`/workflows/${id}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(input),
    });
  },

  deleteWorkflow(id: string): Promise<void> {
    return request(`/workflows/${id}`, { method: "DELETE" });
  },

  runWorkflow(id: string): Promise<{ data: RunResult }> {
    return request(`/workflows/${id}/run`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: "{}",
    });
  },

  listKnots(): Promise<{ data: KnotDescriptor[] }> {
    return request("/knots");
  },
};

export { ApiError };
