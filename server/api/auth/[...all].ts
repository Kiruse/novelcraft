import { auth } from '#server/auth/config';

export default defineEventHandler((event) => auth.handler(toWebRequest(event)));
