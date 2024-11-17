import { DropInfo, Node } from 'flexlayout-react'

export const onAllowDrop = (dragNode: Node, dropInfo: DropInfo) => {
  const dropNode = dropInfo.node

  // prevent non-border tabs dropping into borders
  if (
    dropNode.getType() === 'border' &&
    (dragNode.getParent() == null ||
      dragNode.getParent()!.getType() !== 'border')
  )
    return false

  // prevent border tabs dropping into main layout
  if (
    dropNode.getType() !== 'border' &&
    dragNode.getParent() != null &&
    dragNode.getParent()!.getType() === 'border'
  )
    return false

  return true
}

