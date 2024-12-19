import {
  findAllByAltText,
  fireEvent,
  render,
  waitFor,
} from "@testing-library/svelte";
import { describe, test, expect } from "vitest";
import Home from "../src/routes/Home.svelte";
import Cameras from "../src/routes/Cameras.svelte";
import CameraAndVideos from "../src/lib/components/CameraAndVideos.svelte";
import Login from "../src/routes/Login.svelte";
import { get, Writable } from "svelte/store";
import { socket } from "../src/lib/stores/socketStore";
import { user } from "../src/lib/stores/userStore";
import {
  testCamera1,
  testCameras,
  testPermissions,
  testUserAndCameras,
  timeoutPromise,
} from "../vitest-setup";

// https://testing-library.com/docs/queries/about/

function waitForStoreChange<T>(store: Writable<T>) {
  return new Promise((resolve) => {
    let initialValueReceived = false;

    store.subscribe((value) => {
      if (!initialValueReceived) {
        initialValueReceived = true;
        return;
      }

      resolve(value);
    });
  });
}

describe("Login Flow", () => {
  test("successful login updates user store", async () => {
    const userValue = get(user);
    expect(userValue).toBe(null);

    const { getByText } = render(Login);

    const newUserValuePromise = waitForStoreChange(user);

    await fireEvent.click(getByText("Sign in"));

    const newUserValue = await newUserValuePromise;

    expect(newUserValue).toStrictEqual(testUserAndCameras);
  });
});

describe("Cameras page", () => {
  test("image is displayed when received", async () => {
    const liveFeedAltText = "live feed";

    user.set(testUserAndCameras);
    socket.set(new WebSocket("ws://localhost:3000/api/ws"));

    const { getByAltText } = render(CameraAndVideos, {
      cameraId: 2,
      cameraName: "Kitchen",
    });

    const liveFeedImg = getByAltText(liveFeedAltText);

    expect(liveFeedImg).toBeInTheDocument();
    expect(liveFeedImg.getAttribute("src")).toBe(null);

    let liveFeedImgSrc: string | null = null;

    await waitFor(() => {
      liveFeedImgSrc = liveFeedImg.getAttribute("src");
      expect(liveFeedImgSrc).not.toBe(null);
    });

    await waitFor(() => {
      expect(liveFeedImg.getAttribute("src")).not.toBe(liveFeedImgSrc);
    });
  });

  test("camera list updates when camera is added/removed", async () => {
    user.set(testUserAndCameras);

    const { getByText, queryByText, getAllByLabelText } = render(Cameras);

    expect(queryByText(testCameras[0].camera_name)).not.toBeInTheDocument();

    await waitFor(() => {
      expect(getByText(testCameras[0].camera_name)).toBeInTheDocument();
    });

    await fireEvent.click(getByText("Add Camera"));

    await fireEvent.click(getByText("Submit"));

    await waitFor(() => {
      expect(queryByText(testCamera1.camera_name)).toBeInTheDocument();
    });

    const removeCameraButtons = getAllByLabelText("Remove Camera");
    const removeBackyardButton = removeCameraButtons.find(
      (button) => button.dataset.cameraId === testCamera1.camera_id.toString(),
    );

    expect(removeBackyardButton).not.toBe(undefined);
    expect(removeBackyardButton?.tagName).toBe("BUTTON");

    await fireEvent.click(removeBackyardButton as HTMLButtonElement);

    await waitFor(() => {
      expect(queryByText(testCamera1.camera_name)).not.toBeInTheDocument();
    });
  });

  test("user permissions for camera update after change", async () => {
    user.set(testUserAndCameras);

    const {
      getByText,
      queryByText,
      getAllByLabelText,
      getByRole,
      getAllByRole,
    } = render(Cameras);

    expect(queryByText(testCameras[0].camera_name)).not.toBeInTheDocument();

    await waitFor(() => {
      expect(getByText(testCameras[0].camera_name)).toBeInTheDocument();
    });

    const editCameraButtons = getAllByLabelText("Edit Camera");
    const editBackyardButton = editCameraButtons.find(
      (button) =>
        button.dataset.cameraId === testCameras[0].camera_id.toString(),
    );

    expect(editBackyardButton).not.toBe(undefined);
    expect(editBackyardButton?.tagName).toBe("BUTTON");

    await fireEvent.click(editBackyardButton as HTMLButtonElement);

    await waitFor(() => {
      testPermissions.forEach((permission) => {
        expect(queryByText(permission.username)).toBeInTheDocument();
      });
    });

    const currentPermissionSpans = getAllByLabelText(
      "Current User Camera Permission",
    );
    const currentPermissionSpan = currentPermissionSpans.find(
      (span) =>
        span.dataset.permissionId ===
        testPermissions[1].permission_id.toString(),
    );

    expect(currentPermissionSpan).not.toBe(undefined);
    expect(currentPermissionSpan?.tagName).toBe("SPAN");
    expect(currentPermissionSpan?.textContent).toBe("Viewer");

    const editPermissionButtons = getAllByLabelText(
      "Edit User Camera Permission",
    );
    const editUserPermissionButton = editPermissionButtons.find(
      (button) =>
        button.dataset.permissionId ===
        testPermissions[1].permission_id.toString(),
    );

    expect(editUserPermissionButton).not.toBe(undefined);
    expect(editUserPermissionButton?.tagName).toBe("BUTTON");

    await fireEvent.click(editUserPermissionButton as HTMLButtonElement);

    await waitFor(() => {
      expect(() =>
        getAllByRole("option", { selected: false }),
      ).not.toThrowError();
    });

    const options = getAllByRole("option", { selected: false });
    const option = options.find(
      (option) =>
        option.tagName === "DIV" && option.dataset.value === '"controller"',
    );

    expect(option).not.toBe(undefined);

    await fireEvent.click(option as HTMLOptionElement);

    // Select element changes value immediately and *then* refreshes, so we can't just use waitFor since the value will already be updated
    await timeoutPromise(200);

    await waitFor(() => {
      const updatedCurrentPermissionSpans = getAllByLabelText(
        "Current User Camera Permission",
      );
      const updatedCurrentPermissionSpan = updatedCurrentPermissionSpans.find(
        (span) =>
          span.dataset.permissionId ===
          testPermissions[1].permission_id.toString(),
      );

      expect(updatedCurrentPermissionSpan).not.toBe(undefined);
      expect(updatedCurrentPermissionSpan?.tagName).toBe("SPAN");
      expect(updatedCurrentPermissionSpan?.textContent).toBe("Controller");
    });
  });

  // TODO: Add test for recording list?
});

describe("Home page", () => {
  test("images from different cameras are displayed when received", async () => {
    const liveFeedAltText = "live camera feed";

    user.set(testUserAndCameras);
    socket.set(new WebSocket("ws://localhost:3000/api/ws"));

    const { getByAltText, queryByAltText, getAllByAltText, findAllByAltText } =
      render(Home);

    expect(queryByAltText(liveFeedAltText)).not.toBeInTheDocument();

    let liveFeedImgs: HTMLElement[] | null = null;

    await waitFor(() => {
      liveFeedImgs = getAllByAltText(liveFeedAltText);
      expect(liveFeedImgs).not.toBe(null);
      expect(liveFeedImgs.length).toBe(testCameras.length);
    });

    expect(
      [...liveFeedImgs!].some(
        (img) => (img.dataset.cameraId = testCameras[0].camera_id),
      ),
    ).toBe(true);
    expect(
      [...liveFeedImgs!].some(
        (img) => (img.dataset.cameraId = testCameras[1].camera_id),
      ),
    ).toBe(true);

    await waitFor(() => {
      expect(liveFeedImgs![0].getAttribute("src")).not.toBe(null);
      expect(liveFeedImgs![1].getAttribute("src")).not.toBe(null);
    });

    expect(liveFeedImgs![0].getAttribute("src")).not.toBe(
      liveFeedImgs![1].getAttribute("src"),
    );
  });
});