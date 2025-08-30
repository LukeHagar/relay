import { redirect } from "@sveltejs/kit";
const load = async ({ locals }) => {
  const session = await locals.auth();
  if (!session) throw redirect(302, "/api/auth/signin");
  return {};
};
export {
  load
};
