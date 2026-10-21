import config from './site.json';
export const href = (path = '') => `${config.base}/${path.replace(/^\//, '')}`;
export const repository = `https://github.com/${config.repository}`;
export const releases = `https://github.com/${config.releaseRepository}/releases`;
export const download = `${releases}/download/${config.releaseTag}/${config.assetName}`;
export const guide = (slug = '') => href(`guide/${slug ? `${slug}/` : ''}`);
