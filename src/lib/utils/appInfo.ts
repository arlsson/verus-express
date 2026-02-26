import { getIdentifier, getName, getVersion } from '@tauri-apps/api/app';

export interface RuntimeAppInfo {
  name: string;
  version: string;
  build: string;
}

const FALLBACK_APP_INFO: RuntimeAppInfo = {
  name: 'Verus Express',
  version: '0.1.0',
  build: 'com.maxtheyse.verus-express'
};

export async function loadRuntimeAppInfo(): Promise<RuntimeAppInfo> {
  const [nameResult, versionResult, identifierResult] = await Promise.allSettled([
    getName(),
    getVersion(),
    getIdentifier()
  ]);

  const name =
    nameResult.status === 'fulfilled' && nameResult.value.trim().length > 0
      ? nameResult.value
      : FALLBACK_APP_INFO.name;
  const version =
    versionResult.status === 'fulfilled' && versionResult.value.trim().length > 0
      ? versionResult.value
      : FALLBACK_APP_INFO.version;
  const build =
    identifierResult.status === 'fulfilled' && identifierResult.value.trim().length > 0
      ? identifierResult.value
      : FALLBACK_APP_INFO.build;

  return {
    name,
    version,
    build
  };
}
