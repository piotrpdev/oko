import { writable } from "@macfja/svelte-persistent-store";

export type User = {
  user_id: number;
  username: string;
  password_hash: string;
  created_at: Array<number>;
};

export type Camera = object;

export type UserAndCameras = {
  user: User;
  cameras: Array<Camera>;
};

export const user = writable("user", null as UserAndCameras | null);
