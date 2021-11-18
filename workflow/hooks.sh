#!/bin/zsh

# https://stackoverflow.com/questions/4592838/symbolic-link-to-a-hook-in-git#4594681
ln -si ../../workflow/pre-commit ../.git/hooks/pre-commit &&
ln -si ../../workflow/prepare-commit-msg ../.git/hooks/prepare-commit-msg
if (( $? ))
then    
 echo "ERROR couldn't symlink hooks"
else
 echo "Hooks symlinked!"
fi