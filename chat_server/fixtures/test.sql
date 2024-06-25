-- insert 3 workspaces
INSERT INTO workspaces(name, owner_id)
  VALUES ('ws1', 0),
('ws2', 0),
('ws3', 0);


-- insert 5 users, all with hashed password 'Hunter48'
INSERT INTO users(ws_id, email, fullname, password_hash)
    VALUES(1, 'jack1@gmail.com', 'jack1', '$argon2id$v=19$m=19456,t=2,p=1$B6zbATA/ttJCTVa/P8eJDQ$AwNAtiAxjDFO59RDB4xI2bxD++/eaIFKEkdGaPvVvak'),
    (1, 'jack2@gmail.com', 'jack2', '$argon2id$v=19$m=19456,t=2,p=1$B6zbATA/ttJCTVa/P8eJDQ$AwNAtiAxjDFO59RDB4xI2bxD++/eaIFKEkdGaPvVvak'),
    (1, 'jack3@gmail.com', 'jack3', '$argon2id$v=19$m=19456,t=2,p=1$B6zbATA/ttJCTVa/P8eJDQ$AwNAtiAxjDFO59RDB4xI2bxD++/eaIFKEkdGaPvVvak'),
    (1, 'jack4@gmail.com', 'jack4', '$argon2id$v=19$m=19456,t=2,p=1$B6zbATA/ttJCTVa/P8eJDQ$AwNAtiAxjDFO59RDB4xI2bxD++/eaIFKEkdGaPvVvak'),
    (1, 'jack5@gmail.com', 'jack5', '$argon2id$v=19$m=19456,t=2,p=1$B6zbATA/ttJCTVa/P8eJDQ$AwNAtiAxjDFO59RDB4xI2bxD++/eaIFKEkdGaPvVvak');

-- insert 4 chats
-- insert public/private channel
INSERT INTO chats(ws_id, name, type, members)
  VALUES (1, 'general', 'public_channel', '{1,2,3,4,5}'),
(1, 'private', 'private_channel', '{1,2,3}');

-- insert unnamed chat
INSERT INTO chats(ws_id, type, members)
  VALUES (1, 'single', '{1,2}'),
(1, 'group', '{1,3,4}');
