import { open } from '@tauri-apps/api/dialog';
import { convertFileSrc } from '@tauri-apps/api/tauri';
import { getCanonicalPath, getWorkspacePath } from '../bindings';
import { toastStore, type ToastSettings } from '@skeletonlabs/skeleton';

export async function openImage(): Promise<string | null> {
	const path = await open({
		multiple: false,
		filters: [
			{
				name: 'Image',
				extensions: ['png', 'jpeg', 'jpg', 'webp']
			}
		]
	});
	if (typeof path === 'object') {
		throw Error('Must be single file');
	}
	return path;
}

export async function imageSrc(path: string | null): Promise<string> {
	if (path == null) {
		return '';
	}
	if (path.startsWith('https://') || path.startsWith('http://')) {
		return path;
	}
	const workspace = await getWorkspacePath().catch((err) => {
		const toast: ToastSettings = {
			message: err
		};
		toastStore.trigger(toast);
		return '';
	});
	const canonicalPath = await getCanonicalPath(workspace + '/' + path).catch((err) => {
		const toast: ToastSettings = {
			message: err
		};
		toastStore.trigger(toast);
		return '';
	});
	const image = convertFileSrc(canonicalPath);
	return image;
}
