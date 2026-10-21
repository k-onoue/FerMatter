import config from './site.json';
export const href = (path = '') => `${config.base}/${path.replace(/^\//, '')}`;
export const repository = `https://github.com/${config.repository}`;
export const guide = (slug = '') => href(`guide/${slug ? `${slug}/` : ''}`);
