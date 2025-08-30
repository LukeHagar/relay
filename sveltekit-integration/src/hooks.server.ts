import { handle as authHandle } from '$auth';
import { sequence } from '@sveltejs/kit/hooks';
import type { Handle } from '@sveltejs/kit';

const customHandle: Handle = async ({ event, resolve }) => {
	// Add custom server-side logic here if needed
	return resolve(event);
};

export const handle = sequence(authHandle, customHandle);