export default defineI18nConfig(() => ({
  fallbackLocale: 'en',
  pluralRules: {
    ru: (choice: number) => {
      const tens = Math.abs(choice) % 100
      const units = tens % 10
      if (tens > 10 && tens < 20) return 2
      if (units === 1) return 0
      if (units >= 2 && units <= 4) return 1
      return 2
    },
  },
}))
