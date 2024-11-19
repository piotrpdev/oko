import { writable } from "@macfja/svelte-persistent-store";

export type User = {
  user_id: number;
  username: string;
  password_hash: string;
  created_at: Array<number>;
};

export type Camera = {
  camera_id: number;
  camera_name: string;
  can_control: boolean;
  can_view: boolean;
};

export type UserAndCameras = {
  user: User;
  cameras: Array<Camera>;
};

export const user = writable("user", null as UserAndCameras | null);
