import { ConvexSignalsClient } from 'convex-signals-client';

export const convex = new ConvexSignalsClient(process.env.NEXT_PUBLIC_CONVEX_URL!);
