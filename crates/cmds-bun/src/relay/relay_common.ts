/**
 * Shared helpers for email (Resend) and SMS (Twilio) Bun nodes.
 *
 * Generic API clients — not tied to any specific use case.
 */

/** Resend API: send an email. Returns { id: string }. */
export async function resendSendEmail(params: {
  api_key: string;
  from: string;
  to: string | string[];
  reply_to?: string;
  subject: string;
  text?: string;
  html?: string;
  cc?: string[];
  bcc?: string[];
  headers?: Record<string, string>;
  tags?: Array<{ name: string; value: string }>;
}): Promise<{ id: string }> {
  const toArr = Array.isArray(params.to) ? params.to : [params.to];

  const body: Record<string, any> = {
    from: params.from,
    to: toArr,
    subject: params.subject,
  };
  if (params.reply_to) body.reply_to = params.reply_to;
  if (params.html) body.html = params.html;
  if (params.text) body.text = params.text;
  if (params.cc?.length) body.cc = params.cc;
  if (params.bcc?.length) body.bcc = params.bcc;
  if (params.headers) body.headers = params.headers;
  if (params.tags?.length) body.tags = params.tags;

  const resp = await fetch("https://api.resend.com/emails", {
    method: "POST",
    headers: {
      "Authorization": `Bearer ${params.api_key}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body),
  });

  if (!resp.ok) {
    const err = await resp.text();
    throw new Error(`Resend API error ${resp.status}: ${err}`);
  }

  return resp.json();
}

/** Twilio API: send an SMS. Returns { sid: string, status: string, ... }. */
export async function twilioSendSms(params: {
  account_sid: string;
  auth_token: string;
  from: string;
  to: string;
  body: string;
  status_callback?: string;
  media_url?: string[];
}): Promise<{ sid: string; status: string; date_created: string }> {
  const url = `https://api.twilio.com/2010-04-01/Accounts/${params.account_sid}/Messages.json`;

  const formData = new URLSearchParams();
  formData.set("From", params.from);
  formData.set("To", params.to);
  formData.set("Body", params.body);
  if (params.status_callback) formData.set("StatusCallback", params.status_callback);
  if (params.media_url) {
    for (const url of params.media_url) {
      formData.append("MediaUrl", url);
    }
  }

  const credentials = btoa(`${params.account_sid}:${params.auth_token}`);

  const resp = await fetch(url, {
    method: "POST",
    headers: {
      "Authorization": `Basic ${credentials}`,
      "Content-Type": "application/x-www-form-urlencoded",
    },
    body: formData.toString(),
  });

  if (!resp.ok) {
    const err = await resp.text();
    throw new Error(`Twilio API error ${resp.status}: ${err}`);
  }

  const result = await resp.json();
  return { sid: result.sid, status: result.status, date_created: result.date_created };
}
