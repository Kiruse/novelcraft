import { betterAuth } from 'better-auth';
import { drizzleAdapter } from 'better-auth/adapters/drizzle';
import { magicLink } from 'better-auth/plugins/magic-link';
import { db } from '#server/db';
import { sendMail } from './email';

export const auth = betterAuth({
  database: drizzleAdapter(db, {
    provider: 'pg',
  }),
  user: {
    additionalFields: {
      isAuthor: {
        type: 'boolean',
        defaultValue: false,
      },
    },
  },
  emailAndPassword: {
    enabled: true,
    requireEmailVerification: true,
  },
  emailVerification: {
    sendVerificationEmail: async ({ user, url }) => {
      await sendMail({
        to: user.email,
        subject: 'Verify your email — NovelCraft',
        html: `<p>Hey ${user.name},</p>
<p>Click the link below to verify your email address:</p>
<p><a href="${url}">Verify email</a></p>
<p>If you didn't create an account, you can ignore this email.</p>`,
      });
    },
    sendOnSignUp: true,
  },
  plugins: [
    magicLink({
      sendMagicLink: async ({ email, url }) => {
        await sendMail({
          to: email,
          subject: 'Your sign-in link — NovelCraft',
          html: `<p>Click the link below to sign in:</p>
<p><a href="${url}">Sign in to NovelCraft</a></p>
<p>This link expires in 5 minutes. If you didn't request this, you can ignore this email.</p>`,
        });
      },
    }),
  ],
});
