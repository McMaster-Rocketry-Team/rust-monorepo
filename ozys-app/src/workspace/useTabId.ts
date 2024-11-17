import { createContext, useContext } from 'react'

const TabIdContext = createContext<string | undefined>(undefined)

export const TabIdProvider = TabIdContext.Provider

export function useTabId() {
  return useContext(TabIdContext)!
}