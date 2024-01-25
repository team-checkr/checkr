import { PUBLIC_API_BASE } from '$env/static/public';
import { setGlobalApiBase } from '$lib/api';

export const prerender = true;

setGlobalApiBase(PUBLIC_API_BASE || 'http://0.0.0.0:3000/api');
