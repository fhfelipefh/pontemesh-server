export function isValidAdminPassword(password: string): boolean {
  return Array.from(password).length >= 12
    && /\p{Ll}/u.test(password)
    && /\p{Lu}/u.test(password)
    && /\p{N}/u.test(password)
    && /[^\p{L}\p{N}]/u.test(password);
}
