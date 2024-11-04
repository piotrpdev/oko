import { fireEvent, render } from "@testing-library/svelte";
import { describe, test, expect } from "vitest";
import Login from "../src/routes/Login.svelte";
import { get, Writable } from "svelte/store";
import { user } from "../src/lib/userStore";
import { testUser } from "../vitest-setup";

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

    await fireEvent.click(getByText("Login"));

    const newUserValue = await newUserValuePromise;

    expect(newUserValue).toStrictEqual(testUser);
  });
});
