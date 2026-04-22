import nodemailer from 'nodemailer';

const SMTP_HOST = process.env.SMTP_HOST ?? 'localhost';
const SMTP_PORT = parseInt(process.env.SMTP_PORT ?? '10025', 10);
const SMTP_USER = process.env.SMTP_USER ?? '';
const SMTP_PASS = process.env.SMTP_PASS ?? '';
const SMTP_FROM = process.env.SMTP_FROM ?? 'noreply@novelcraft.quest';

const transport = nodemailer.createTransport({
  host: SMTP_HOST,
  port: SMTP_PORT,
  secure: false,
  tls: {
    rejectUnauthorized: process.env.NODE_ENV === 'production',
  },
  ...(SMTP_USER && SMTP_PASS ? { auth: { user: SMTP_USER, pass: SMTP_PASS } } : {}),
});

export interface MailMessage {
  to: string;
  subject: string;
  html: string;
}

export async function sendMail(message: MailMessage): Promise<void> {
  await transport.sendMail({
    from: SMTP_FROM,
    to: message.to,
    subject: message.subject,
    html: message.html,
  });
}
