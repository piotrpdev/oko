import { fireEvent, render, waitFor } from "@testing-library/svelte";
import { describe, test, expect } from "vitest";
import Home from "../src/routes/Home.svelte";
import Login from "../src/routes/Login.svelte";
import { get, Writable } from "svelte/store";
import { user } from "../src/lib/userStore";
import { testUserAndCameras } from "../vitest-setup";

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

describe("WebSocket Image Handling", () => {
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
});
