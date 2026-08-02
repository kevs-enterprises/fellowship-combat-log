# Publish workflow smoke test

This file exists only to verify the publish-to-public workflow end-to-end.
Safe to delete — it lives only on a throwaway test branch, never on main.

This line intentionally contains a fake AWS key to verify the secret scanner blocks it:
aws_access_key_id = AKIAIOSFODNN7EXAMPLE
aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
