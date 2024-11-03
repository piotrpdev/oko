import { writable } from 'svelte/store';

type User = {
    user_id: number,
    username: string,
    password_hash: string,
    created_at: Array<number>,
}

export const user = writable(null as User | null);