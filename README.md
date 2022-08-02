## TODO

make bot context sensitive for each person: save the context somewhere and switch it on the fly

do not allow the bot to answer two questions in the voice at the same time

text to speech does not return the data field for texts that are too long(maybe we can create the audio in 2 steps - divide by punctuation)

The text bot generates messages prefixed by Me: sometimes. We could remove it since it makes the answer look weird.

When you mention someone, the conversation bot uses the discord id instead of the user name, fix it.

Retry google translate call when it returns an unexpected response. (see if we are being rate limited)

Tts text can have 200 characters at max.
