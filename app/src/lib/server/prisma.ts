let prismaSingleton: any | undefined;

export async function getPrisma() {
	if (!prismaSingleton) {
		const { PrismaClient } = await import('@prisma/client');
		prismaSingleton = new PrismaClient();
	}
	return prismaSingleton as import('@prisma/client').PrismaClient;
}

