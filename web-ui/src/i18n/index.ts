export type Locale = 'en' | 'es' | 'de' | 'ja' | 'zh';

type TranslationStrings = Record<string, string>;

export const translations: Record<Locale, TranslationStrings> = {
  en: {
    dashboard: 'Dashboard',
    flows: 'Flows',
    policies: 'Policies',
    settings: 'Settings',
    search: 'Search',
    refresh: 'Refresh',
    save: 'Save',
    cancel: 'Cancel',
    delete: 'Delete',
    create: 'Create',
    loading: 'Loading...',
    error: 'Error',
    success: 'Success',
    noData: 'No data available',
    topology: 'Topology',
    anomalies: 'Anomalies',
    compliance: 'Compliance',
    events: 'Events',
    nodes: 'Nodes',
    endpoints: 'Endpoints',
    export: 'Export',
    import: 'Import',
    filter: 'Filter',
    sort: 'Sort',
    actions: 'Actions',
    status: 'Status',
    name: 'Name',
    namespace: 'Namespace',
    source: 'Source',
    destination: 'Destination',
    protocol: 'Protocol',
    port: 'Port',
    verdict: 'Verdict',
    timestamp: 'Timestamp',
    apply: 'Apply',
    reset: 'Reset',
    close: 'Close',
    confirm: 'Confirm',
    edit: 'Edit',
    view: 'View',
    download: 'Download',
    upload: 'Upload',
    enabled: 'Enabled',
    disabled: 'Disabled',
    all: 'All',
    none: 'None',
    total: 'Total',
    showing: 'Showing',
    of: 'of',
    results: 'results',
    previous: 'Previous',
    next: 'Next',
    home: 'Home',
    notifications: 'Notifications',
    darkMode: 'Dark Mode',
    lightMode: 'Light Mode',
    signOut: 'Sign Out',
    version: 'Version',
    connection: 'Connection',
    connected: 'Connected',
    disconnected: 'Disconnected',
  },
  es: {},
  de: {},
  ja: {},
  zh: {},
};

const DEFAULT_LOCALE: Locale = 'en';

export function t(key: string, locale: Locale = DEFAULT_LOCALE): string {
  const localeStrings = translations[locale];
  if (localeStrings && key in localeStrings) {
    return localeStrings[key];
  }
  // Fall back to English
  if (locale !== DEFAULT_LOCALE) {
    const fallback = translations[DEFAULT_LOCALE];
    if (key in fallback) {
      return fallback[key];
    }
  }
  // Return key itself as last resort
  return key;
}
