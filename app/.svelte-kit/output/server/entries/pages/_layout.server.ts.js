const load = async ({ locals }) => {
  const session = await locals.auth();
  return { session };
};
export {
  load
};
