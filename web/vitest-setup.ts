import "@testing-library/jest-dom/vitest";
import { afterAll, afterEach, beforeAll } from "vitest";
import { setupServer } from "msw/node";
import { http, HttpResponse } from "msw";
import { User } from "./src/lib/userStore.ts";

// TODO: Use generated OpenAPI spec and types

export const testUser: User = {
  user_id: 1,
  username: "admin",
  password_hash: "[redacted]",
  created_at: [2021, 10, 21, 17, 1, 23],
};

export const restHandlers = [
  http.post("/api/login", () => {
    return new Response(null, {
      status: 200,
    });
  }),
  http.get("/api/", () => {
    return HttpResponse.json(testUser);
  }),
];

const server = setupServer(...restHandlers);

beforeAll(() => server.listen({ onUnhandledRequest: "error" }));
afterAll(() => server.close());
afterEach(() => server.resetHandlers());
