# UnityEngine Meta Git Differ

Working with Unity GUIDs is not always easy. When changing the file path of a file or straight up moving/renaming one, it is of advantage to preserve the associated UUIDs.  
This tool scans for GUID path related changes in Git commits and work directory. It will highlight path changes and GUID additions/removals.

You can supply two commit hashes (short or long form) to create and debug a diff between them. Only supplying hash will compare that commit with the current work directory state. No commit compares the head commit with the work directory state.

Currently, the tool will print the file tree twice. Once for removed and once for added files. This allows to check changes in both directions and makes them obvious.
