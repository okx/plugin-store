import Redis from 'ioredis';

const redis = new Redis(process.env.REDIS_URL);

// Admin endpoint: remove stats entries not in the allowed list.
// POST /api/cleanup { "keep": ["plugin-a", "plugin-b"], "token": "..." }
export default async function handler(req, res) {
  if (req.method !== 'POST') {
    return res.status(405).json({ error: 'Method not allowed' });
  }

  const { keep, token } = req.body ?? {};

  if (token !== process.env.ADMIN_TOKEN) {
    return res.status(403).json({ error: 'Forbidden' });
  }

  if (!Array.isArray(keep) || keep.length === 0) {
    return res.status(400).json({ error: 'Missing keep array' });
  }

  const all = await redis.hgetall('installs') ?? {};
  const keepSet = new Set(keep);
  const removed = [];

  for (const name of Object.keys(all)) {
    if (!keepSet.has(name)) {
      await redis.hdel('installs', name);
      removed.push(name);
    }
  }

  return res.status(200).json({ ok: true, removed, remaining: keep.length });
}
