ALTER POLICY "authenticated-update" ON nodes TO authenticated USING (auth.uid() = user_id);
