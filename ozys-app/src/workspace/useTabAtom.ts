import { atomFamily, atomWithStorage } from 'jotai/utils'
import { useTabId } from './useTabId'
import { useAtom } from 'jotai';

const tabAtomFamily = atomFamily(
  (config: { tabId: string; key: string; defaultValue: any }) =>
    atomWithStorage(`${config.tabId}-${config.key}`, config.defaultValue),
  (a, b) => a.tabId === b.tabId && a.key === b.key
)

export const useTabAtom = <T>(key: string, defaultValue: T) => {
  const tabId = useTabId()
  const atom = tabAtomFamily({ tabId, key, defaultValue })

  return useAtom<T>(atom)
}
