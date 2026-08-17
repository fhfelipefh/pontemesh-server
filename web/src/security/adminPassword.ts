export const MAX_ADMIN_PASSWORD_LENGTH = 256;

export function isValidAdminPassword(password: string): boolean {
  const length = Array.from(password).length;
  return length >= 12
    && length <= MAX_ADMIN_PASSWORD_LENGTH
    && /\p{Ll}/u.test(password)
    && /\p{Lu}/u.test(password)
    && /\p{N}/u.test(password)
    && /[^\p{L}\p{N}]/u.test(password);
}
