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

export type VideoCameraView = {
  video_id: number;
  camera_id: number;
  camera_name: string;
  file_path: string;
  file_size: number;
};

export type ImageContainer = {
  camera_id: number;
  timestamp: number;
  image_bytes: Array<number>;
};

export function isImageContainer(obj: unknown): obj is ImageContainer {
  return (
    obj instanceof Object &&
    "camera_id" in obj &&
    "timestamp" in obj &&
    "image_bytes" in obj &&
    typeof obj.camera_id === "number" &&
    typeof obj.timestamp === "number" &&
    Array.isArray(obj.image_bytes)
  );
}
