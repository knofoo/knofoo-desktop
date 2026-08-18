// src/routes/settings/+page.ts
import { redirect } from '@sveltejs/kit';

export function load() {
    throw redirect(302, '/settings/general');
}