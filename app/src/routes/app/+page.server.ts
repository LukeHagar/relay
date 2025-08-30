import { redirect } from '@sveltejs/kit';

export const load = async ({ locals }) => {
	const session = await locals.auth();
	if (!session) throw redirect(302, '/api/auth/signin');
	return {};
};

