---
trigger: always_on
---

# No Scratch Files in Version Control

You must NEVER commit temporary, scratch, or helper scripts (such as Python scripts you created to make quick edits, sync files, test APIs, etc.) to the project repository or pull requests.
Before running `git add -A` or any commit command, ALWAYS check the `git status` or explicitly define the files to be added to ensure that only the requested source code files are included.
If you use a scratch file, you must delete it or ensure it is untracked before committing.
