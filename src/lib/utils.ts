const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ';

export function numToAlpha(num: number) {
	let result = '';
	num++;

	while (num > 0) {
		const index = (num - 1) % 26;
		result = alphabet[index] + result;
		num = Math.floor((num - 1) / 26);
	}

	return result;
}
