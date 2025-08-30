import { handle as authHandle } from '$lib/auth';
import { sequence } from '@sveltejs/kit/hooks';

export const handle = sequence(authHandle);