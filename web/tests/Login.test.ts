import { fireEvent, render, waitFor } from "@testing-library/svelte";
import { describe, test, expect } from "vitest";
import Home from "../src/routes/Home.svelte";
import Login from "../src/routes/Login.svelte";
import { get, Writable } from "svelte/store";
import { user } from "../src/lib/userStore";
import { testCamera1, testCameras, testUserAndCameras } from "../vitest-setup";

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

    const { getByAltText } = render(Home);

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

    const { getByText, queryByText, getAllByLabelText } = render(Home);

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

  // TODO: Add test for recording list?
});
