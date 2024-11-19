import "@testing-library/jest-dom/vitest";
import { afterAll, afterEach, beforeAll } from "vitest";
import { setupServer } from "msw/node";
import { http, HttpResponse, ws } from "msw";
import { WebSocket } from "undici";
import { User } from "./src/lib/userStore.ts";

Reflect.set(globalThis, "WebSocket", WebSocket);

// TODO: Use generated OpenAPI spec and types
const api_ws = ws.link("ws://localhost:3000/api/ws");

export const testUser: User = {
  user_id: 1,
  username: "admin",
  password_hash: "[redacted]",
  created_at: [2021, 10, 21, 17, 1, 23],
};

export const testUserAndCameras = {
  user: testUser,
  cameras: [],
};

function timeoutPromise(ms: number) {
  return new Promise((res) => setTimeout(res, ms));
}

export const testBlob1 = new Blob([new Uint8Array([1])], { type: "image/jpg" });
export const testBlob2 = new Blob([new Uint8Array([2])], { type: "image/jpg" });

export const handlers = [
  http.post("/api/login", () => {
    return new Response(null, {
      status: 200,
    });
  }),
  http.get("/api/", () => {
    return HttpResponse.json(testUserAndCameras);
  }),
  api_ws.addEventListener("connection", async ({ client }) => {
    console.log("WebSocket connection established");
    // https://stackoverflow.com/a/16245768/19020549
    client.send(testBlob1);
    await timeoutPromise(10);
    client.send(testBlob2);
  }),
];

const server = setupServer(...handlers);

beforeAll(() => {
  // global.WebSocket = null;
  server.listen({ onUnhandledRequest: "error" });
});
afterAll(() => server.close());
afterEach(() => server.resetHandlers());
