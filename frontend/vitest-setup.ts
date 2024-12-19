import "@testing-library/jest-dom/vitest";
import { afterAll, afterEach, beforeAll, beforeEach } from "vitest";
import { setupServer } from "msw/node";
import { http, HttpResponse, ws } from "msw";
import { WebSocket } from "undici";
import {
  Camera,
  CameraPermission,
  ImageContainer,
  User,
  VideoCameraView,
} from "./src/types.ts";

Reflect.set(globalThis, "WebSocket", WebSocket);

// TODO: Use generated OpenAPI spec and types
const api_ws = ws.link("ws://localhost:3000/api/ws");

export const testUser: User = {
  user_id: 1,
  username: "admin",
  password_hash: "[redacted]",
  created_at: [2021, 10, 21, 17, 1, 23],
};

export let testCameras: Camera[] = [
  {
    camera_id: 1,
    camera_name: "Front Door",
    can_control: true,
    can_view: true,
  },
  {
    camera_id: 2,
    camera_name: "Kitchen",
    can_control: true,
    can_view: true,
  },
];

export const testCamera1: Camera = {
  camera_id: 3,
  camera_name: "Backyard",
  can_control: true,
  can_view: true,
};

export const testUserAndCameras = {
  user: testUser,
  cameras: testCameras,
};

export const testVideos: VideoCameraView[] = [
  {
    video_id: 2,
    camera_id: 2,
    camera_name: "Kitchen",
    file_path: "2.mp4",
    file_size: 6905856,
  },
];

export const testPermissions: CameraPermission[] = [
  {
    permission_id: 3,
    camera_id: 1,
    user_id: 2,
    username: "piotrpdev",
    can_view: true,
    can_control: true,
  },
  {
    permission_id: 5,
    camera_id: 1,
    user_id: 3,
    username: "joedaly",
    can_view: true,
    can_control: false,
  },
];

export function timeoutPromise(ms: number) {
  return new Promise((res) => setTimeout(res, ms));
}

export const testBlob1 = new Blob([new Uint8Array([1])], { type: "image/jpg" });
export const testBlob2 = new Blob([new Uint8Array([2])], { type: "image/jpg" });

export const testImgContainer1: ImageContainer = {
  camera_id: 1,
  timestamp: 1634876400,
  image_bytes: [1],
};

export const testImgContainer2: ImageContainer = {
  camera_id: 2,
  timestamp: 1634876400,
  image_bytes: [2],
};

export const handlers = [
  http.post("/api/login", () => {
    return new Response(null, {
      status: 200,
    });
  }),
  http.get("/api/", () => {
    return HttpResponse.json(testUserAndCameras);
  }),
  http.get("/api/cameras", () => {
    return HttpResponse.json(testCameras);
  }),
  http.get("/api/cameras/:cameraId/videos", ({ params: { cameraId } }) => {
    const parsedCameraId = Number(cameraId);

    const videos = testVideos.filter(
      (video) => video.camera_id == parsedCameraId,
    );

    return HttpResponse.json(videos);
  }),
  http.get("/api/cameras/:cameraId/permissions", ({ params: { cameraId } }) => {
    const parsedCameraId = Number(cameraId);

    const permissions = testPermissions.filter(
      (permission) => permission.camera_id == parsedCameraId,
    );

    return HttpResponse.json(permissions);
  }),
  http.post("/api/cameras", async ({ request }) => {
    const requestBody = await request.formData();

    if (!requestBody) return HttpResponse.error();

    const newCamera = {
      camera_id: testCameras.length + 1,
      camera_name: requestBody.get("name") as string,
      can_control: false,
      can_view: true,
    };

    testCameras.push(newCamera);

    return HttpResponse.json(newCamera.camera_id);
  }),
  // request body: { can_view: string; can_control: string; }
  http.patch(
    "/api/permissions/:permissionId",
    async ({ request, params: { permissionId } }) => {
      const requestBody = await request.formData();

      if (!requestBody) return HttpResponse.error();

      const parsedPermissionId = Number(permissionId);

      const permission = testPermissions.find(
        (permission) => permission.permission_id === parsedPermissionId,
      );

      if (!permission) return HttpResponse.error();

      permission.can_view = requestBody.get("can_view") === "true";
      permission.can_control = requestBody.get("can_control") === "true";

      return HttpResponse.json(permission);
    },
  ),
  http.delete("/api/cameras/:cameraId", ({ params: { cameraId } }) => {
    const parsedCameraId = Number(cameraId);

    testCameras = testCameras.filter(
      (camera) => camera.camera_id !== parsedCameraId,
    );

    return HttpResponse.json(parsedCameraId);
  }),
  api_ws.addEventListener("connection", async ({ client }) => {
    console.log("WebSocket connection established");
    // https://stackoverflow.com/a/16245768/19020549
    testImgContainer1.camera_id = 2;
    client.send(JSON.stringify(testImgContainer1));
    // ? There might be a better way that doesn't involve waiting
    await timeoutPromise(100);
    client.send(JSON.stringify(testImgContainer2));
    await timeoutPromise(100);
    testImgContainer1.camera_id = 1;
    await timeoutPromise(100);
    testImgContainer2.camera_id = 1;
    client.send(JSON.stringify(testImgContainer2));
  }),
];

const server = setupServer(...handlers);

let testCamerasBackup: Camera[];

beforeAll(() => {
  // global.WebSocket = null;
  server.listen({ onUnhandledRequest: "error" });
});
beforeEach(() => {
  testCamerasBackup = [...testCameras];
});
afterAll(() => server.close());
afterEach(() => {
  server.resetHandlers();
  testCameras = testCamerasBackup;
});
